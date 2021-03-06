//! OVSDB JSON interface to RunningProgram
#![cfg(all(feature = "ovsdb", feature = "c_api"))]

use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;
use std::sync;

use ddlog_ovsdb_adapter::*;
use differential_datalog::api::HDDlog;
use differential_datalog::ddval::*;
use differential_datalog::program::*;
use differential_datalog::record::{IntoRecord, Record, UpdCmd};
use differential_datalog::DeltaMap;
use differential_datalog::{DDlog, DDlogDynamic, DDlogInventory};

use crate::Relations;

/// Parse OVSDB JSON <table-updates> value into DDlog commands; apply commands to a DDlog program.
///
/// Must be called in the context of a transaction.
///
/// `prefix` contains is the prefix to be added to JSON table names, e.g, `OVN_Southbound_` or
/// `OVN_Northbound_` for OVN southbound and northbound database updates.
///
/// `updates` is the JSON string, e.g.:
///
/// {"Logical_Switch":{"ffe8d84e-b4a0-419e-b865-19f151eed878":{"new":{"acls":["set",[]],"dns_records":["set",[]],"external_ids":["map",[]],"load_balancer":["set",[]],"name":"lsw0","other_config":["map",[]],"ports":["set",[]],"qos_rules":["set",[]]}}}}
///
///
#[no_mangle]
pub unsafe extern "C" fn ddlog_apply_ovsdb_updates(
    prog: *const HDDlog,
    prefix: *const c_char,
    updates: *const c_char,
) -> c_int {
    if prog.is_null() || prefix.is_null() || updates.is_null() {
        return -1;
    };
    let prog = sync::Arc::from_raw(prog);
    let res = apply_updates(&prog, prefix, updates)
        .map(|_| 0)
        .unwrap_or_else(|e| {
            prog.eprintln(&format!("ddlog_apply_ovsdb_updates(): error: {}", e));
            -1
        });
    sync::Arc::into_raw(prog);
    res
}

fn apply_updates(
    prog: &sync::Arc<HDDlog>,
    prefix: *const c_char,
    updates_str: *const c_char,
) -> Result<(), String> {
    let prefix: &str = unsafe { CStr::from_ptr(prefix) }
        .to_str()
        .map_err(|e| format!("invalid UTF8 string in prefix: {}", e))?;
    let updates_str: &str = unsafe { CStr::from_ptr(updates_str) }
        .to_str()
        .map_err(|e| format!("invalid UTF8 string in prefix: {}", e))?;
    let commands = cmds_from_table_updates_str(prefix, updates_str)?;

    let updates: Result<Vec<Update<DDValue>>, String> = commands
        .iter()
        .map(|c| prog.convert_update_command(c))
        .collect();
    prog.apply_updates(&mut updates?.into_iter())
}

/// Dump OVSDB Delta-Plus, Delta-Minus, and Delta-Update tables as a sequence of OVSDB
/// commands in JSON format.
///
/// On success, returns `0` and stores a pointer to JSON string in `json`.  This pointer must be
/// later deallocated by calling `ddlog_free_json()`
///
/// On error, returns a negative number and writes error message to stderr.
#[no_mangle]
pub unsafe extern "C" fn ddlog_dump_ovsdb_delta_tables(
    prog: *const HDDlog,
    delta: *const DeltaMap<DDValue>,
    module: *const c_char,
    table: *const c_char,
    json: *mut *mut c_char,
) -> c_int {
    if json.is_null() || prog.is_null() || delta.is_null() || module.is_null() || table.is_null() {
        return -1;
    };
    let prog = sync::Arc::from_raw(prog);
    let res = match dump_delta(&prog, &*delta, module, table) {
        Ok(json_string) => {
            *json = json_string.into_raw();
            0
        }
        Err(e) => {
            prog.eprintln(&format!("ddlog_dump_ovsdb_delta_tables(): error: {}", e));
            -1
        }
    };
    sync::Arc::into_raw(prog);
    res
}

unsafe fn dump_delta(
    prog: &HDDlog,
    delta: &DeltaMap<DDValue>,
    module: *const c_char,
    table: *const c_char,
) -> Result<CString, String> {
    let table_str: &str = CStr::from_ptr(table)
        .to_str()
        .map_err(|e| format!("{}", e))?;
    let module_str: &str = CStr::from_ptr(module)
        .to_str()
        .map_err(|e| format!("{}", e))?;

    let table_name = format!("{}::{}", module_str, table_str);
    let plus_table_name = format!("{}::DeltaPlus_{}", module_str, table_str);
    let minus_table_name = format!("{}::DeltaMinus_{}", module_str, table_str);
    let upd_table_name = format!("{}::Update_{}", module_str, table_str);
    // Namespace fun: We would like to obtain the OVSDB table name, which is
    // either the local name of the relation (`table_str`) if the relation does
    // not have an `original` attribute, or the value of `original` otherwise.
    // Since there is no direct way to check if the relation has this attribute,
    // we check if `get_table_original_name()` is different from the fully
    // qualified table name.
    let original_table_name = prog.inventory.get_table_original_name(&table_name)?;
    let ovsdb_table_name = if original_table_name == &table_name {
        table_str
    } else {
        original_table_name
    };

    /* DeltaPlus */
    let plus_cmds: Result<Vec<String>, String> = {
        let plus_table_id = Relations::try_from(plus_table_name.as_str())
            .map_err(|()| format!("unknown table {}", plus_table_name))?;

        delta.try_get_rel(plus_table_id as RelId).map_or_else(
            || Ok(vec![]),
            |rel| {
                rel.iter()
                    .map(|(v, w)| {
                        assert!(*w == 1);
                        record_into_insert_str(v.clone().into_record(), ovsdb_table_name)
                    })
                    .collect()
            },
        )
    };
    let plus_cmds = plus_cmds?;

    /* DeltaMinus */
    let minus_cmds: Result<Vec<String>, String> = {
        match Relations::try_from(minus_table_name.as_str()) {
            Ok(minus_table_id) => delta.try_get_rel(minus_table_id as RelId).map_or_else(
                || Ok(vec![]),
                |rel| {
                    rel.iter()
                        .map(|(v, w)| {
                            assert!(*w == 1);
                            record_into_delete_str(v.clone().into_record(), ovsdb_table_name)
                        })
                        .collect()
                },
            ),
            Err(()) => Ok(vec![]),
        }
    };
    let mut minus_cmds = minus_cmds?;

    /* Update */
    let upd_cmds: Result<Vec<String>, String> = {
        match Relations::try_from(upd_table_name.as_str()) {
            Ok(upd_table_id) => delta.try_get_rel(upd_table_id as RelId).map_or_else(
                || Ok(vec![]),
                |rel| {
                    rel.iter()
                        .map(|(v, w)| {
                            assert!(*w == 1);
                            record_into_update_str(v.clone().into_record(), ovsdb_table_name)
                        })
                        .collect()
                },
            ),
            Err(()) => Ok(vec![]),
        }
    };
    let mut upd_cmds = upd_cmds?;

    let mut cmds = plus_cmds;
    cmds.append(&mut minus_cmds);
    cmds.append(&mut upd_cmds);
    Ok(CString::from_vec_unchecked(cmds.join(",").into_bytes()))
}

#[no_mangle]
pub unsafe extern "C" fn ddlog_into_ovsdb_insert_str(
    prog: *const HDDlog,
    table: *const c_char,
    rec: *const Record,
    json: *mut *mut c_char,
) -> c_int {
    if prog.is_null() || table.is_null() {
        return -1;
    };
    let rec = match rec.as_ref() {
        Some(record) => record,
        _ => return -1,
    };
    let prog = sync::Arc::from_raw(prog);
    let res = match into_insert_str(&prog, table, rec) {
        Ok(json_string) => {
            *json = json_string.into_raw();
            0
        }
        Err(e) => {
            prog.eprintln(&format!("ddlog_into_insert_str(): error: {}", e));
            -1
        }
    };
    sync::Arc::into_raw(prog);
    res
}

fn into_insert_str(prog: &HDDlog, table: *const c_char, rec: &Record) -> Result<CString, String> {
    let table_str: &str = unsafe { CStr::from_ptr(table) }
        .to_str()
        .map_err(|e| format!("{}", e))?;
    record_into_insert_str(rec.clone(), table_str)
        .map(|s| unsafe { CString::from_vec_unchecked(s.into_bytes()) })
}

#[no_mangle]
pub unsafe extern "C" fn ddlog_into_osvdb_delete_str(
    prog: *const HDDlog,
    table: *const c_char,
    rec: *const Record,
    json: *mut *mut c_char,
) -> c_int {
    if prog.is_null() || table.is_null() {
        return -1;
    };
    let rec = match rec.as_ref() {
        Some(record) => record,
        _ => return -1,
    };
    let prog = sync::Arc::from_raw(prog);
    let res = match into_delete_str(&prog, table, rec) {
        Ok(json_string) => {
            *json = json_string.into_raw();
            0
        }
        Err(e) => {
            prog.eprintln(&format!("ddlog_into_delete_str(): error: {}", e));
            -1
        }
    };
    sync::Arc::into_raw(prog);
    res
}

unsafe fn into_delete_str(
    prog: &HDDlog,
    table: *const c_char,
    rec: &Record,
) -> Result<CString, String> {
    let table_str: &str = CStr::from_ptr(table)
        .to_str()
        .map_err(|e| format!("{}", e))?;
    record_into_delete_str(rec.clone(), table_str)
        .map(|s| CString::from_vec_unchecked(s.into_bytes()))
}

#[no_mangle]
pub unsafe extern "C" fn ddlog_into_ovsdb_update_str(
    prog: *const HDDlog,
    table: *const c_char,
    rec: *const Record,
    json: *mut *mut c_char,
) -> c_int {
    if prog.is_null() || table.is_null() {
        return -1;
    };
    let rec = match rec.as_ref() {
        Some(record) => record,
        _ => return -1,
    };
    let prog = sync::Arc::from_raw(prog);
    let res = match into_update_str(&prog, table, rec) {
        Ok(json_string) => {
            *json = json_string.into_raw();
            0
        }
        Err(e) => {
            prog.eprintln(&format!("ddlog_into_update_str(): error: {}", e));
            -1
        }
    };
    sync::Arc::into_raw(prog);
    res
}

fn into_update_str(prog: &HDDlog, table: *const c_char, rec: &Record) -> Result<CString, String> {
    let table_str: &str = unsafe { CStr::from_ptr(table) }
        .to_str()
        .map_err(|e| format!("{}", e))?;
    record_into_update_str(rec.clone(), table_str)
        .map(|s| unsafe { CString::from_vec_unchecked(s.into_bytes()) })
}

#[no_mangle]
pub unsafe extern "C" fn ddlog_dump_ovsdb_output_table(
    prog: *const HDDlog,
    delta: *const DeltaMap<DDValue>,
    module: *const c_char,
    table: *const c_char,
    json: *mut *mut c_char,
) -> c_int {
    if json.is_null() || prog.is_null() || delta.is_null() || module.is_null() || table.is_null() {
        return -1;
    };
    let prog = sync::Arc::from_raw(prog);
    let res = match dump_output(&prog, &*delta, module, table) {
        Ok(json_string) => {
            *json = json_string.into_raw();
            0
        }
        Err(e) => {
            prog.eprintln(&format!("ddlog_dump_ovsdb_output_table(): error: {}", e));
            -1
        }
    };
    sync::Arc::into_raw(prog);
    res
}

unsafe fn dump_output(
    prog: &HDDlog,
    delta: &DeltaMap<DDValue>,
    module: *const c_char,
    table: *const c_char,
) -> Result<CString, String> {
    let table_str: &str = CStr::from_ptr(table)
        .to_str()
        .map_err(|e| format!("{}", e))?;
    let module_str: &str = CStr::from_ptr(module)
        .to_str()
        .map_err(|e| format!("{}", e))?;
    let table_name = format!("{}::Out_{}", module_str, table_str);

    let original_table_name = prog.inventory.get_table_original_name(&table_name)?;
    let ovsdb_table_name = if original_table_name == &table_name {
        table_str
    } else {
        original_table_name
    };

    /* DeltaPlus */
    let cmds: Result<Vec<String>, String> = {
        let table_id = Relations::try_from(table_name.as_str())
            .map_err(|()| format!("unknown table {}", table_name))?;

        delta.try_get_rel(table_id as RelId).map_or_else(
            || Ok(vec![]),
            |rel| {
                rel.iter()
                    .map(|(v, w)| {
                        assert!(*w == 1 || *w == -1);
                        let record = v.clone().into_record();
                        let get_table_name = |table| prog.inventory.get_table_original_name(table);
                        if (*w == 1) {
                            record_into_insert_str(record, ovsdb_table_name)
                        } else {
                            record_into_delete_str(record, ovsdb_table_name)
                        }
                    })
                    .collect()
            },
        )
    };
    let cmds = cmds?;

    Ok(CString::from_vec_unchecked(cmds.join(",").into_bytes()))
}

/// Deallocates strings returned by other functions in this API.
#[no_mangle]
pub unsafe extern "C" fn ddlog_free_json(str: *mut c_char) {
    if str.is_null() {
        return;
    }
    let _ = CString::from_raw(str);
}

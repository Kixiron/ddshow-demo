#![allow(
    unused_imports,
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    unused_parens,
    non_shorthand_field_patterns,
    dead_code,
    overflowing_literals,
    unreachable_patterns,
    unused_variables,
    clippy::missing_safety_doc,
    clippy::toplevel_ref_arg,
    clippy::double_parens,
    clippy::clone_on_copy,
    clippy::just_underscores_and_digits,
    clippy::match_single_binding,
    clippy::op_ref,
    clippy::nonminimal_bool,
    clippy::redundant_clone
)]

mod inventory;
pub mod ovsdb_api;

pub use inventory::{D3logInventory, Inventory};

use num::bigint::BigInt;
use std::convert::TryFrom;
use std::hash::Hash;
use std::ops::Deref;
use std::result;
use std::{any::TypeId, sync};

use ordered_float::*;

use differential_dataflow::collection;
use timely::communication;
use timely::dataflow::scopes;
use timely::worker;

use differential_datalog::ddval::*;
use differential_datalog::program;
use differential_datalog::record;
use differential_datalog::record::FromRecord;
use differential_datalog::record::IntoRecord;
use differential_datalog::record::RelIdentifier;
use differential_datalog::record::UpdCmd;
use differential_datalog::D3logLocationId;
use num_traits::cast::FromPrimitive;
use num_traits::identities::One;
use once_cell::sync::Lazy;

use fnv::{FnvBuildHasher, FnvHashMap};

use serde::ser::SerializeTuple;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;

// This import is only needed to convince the OS X compiler to export
// `extern C` functions declared in ddlog_log.rs in the generated lib.
#[doc(hidden)]
#[cfg(feature = "c_api")]
pub use ddlog_log as hidden_ddlog_log;
#[doc(hidden)]
#[cfg(feature = "c_api")]
pub use differential_datalog::api as hidden_ddlog_api;

/* Wrapper around `Update<DDValue>` type that implements `Serialize` and `Deserialize`
 * traits.  It is currently only used by the distributed_ddlog crate in order to
 * serialize updates before sending them over the network and deserializing them on the
 * way back.  In other scenarios, the user either creates a `Update<DDValue>` type
 * themselves (when using the strongly typed DDlog API) or deserializes `Update<DDValue>`
 * from `Record` using `DDlogConvert::updcmd2upd()`.
 *
 * Why use a wrapper instead of implementing the traits for `Update<DDValue>` directly?
 * `Update<>` and `DDValue` types are both declared in the `differential_datalog` crate,
 * whereas the `Deserialize` implementation is program-specific and must be in one of the
 * generated crates, so we need a wrapper to avoid creating an orphan `impl`.
 *
 * Serialized representation: we currently only serialize `Insert` and `DeleteValue`
 * commands, represented in serialized form as (polarity, relid, value) tuple.  This way
 * the deserializer first reads relid and uses it to decide which value to deserialize
 * next.
 *
 * `impl Serialize` - serializes the value by forwarding `serialize` call to the `DDValue`
 * object (in fact, it is generic and could be in the `differential_datalog` crate, but we
 * keep it here to make it easier to keep it in sync with `Deserialize`).
 *
 * `impl Deserialize` - gets generated in `Compile.hs` using the macro below.  The macro
 * takes a list of `(relid, type)` and generates a match statement that uses type-specific
 * `Deserialize` for each `relid`.
 */
#[derive(Debug)]
pub struct UpdateSerializer(program::Update<DDValue>);

impl From<program::Update<DDValue>> for UpdateSerializer {
    fn from(u: program::Update<DDValue>) -> Self {
        UpdateSerializer(u)
    }
}
impl From<UpdateSerializer> for program::Update<DDValue> {
    fn from(u: UpdateSerializer) -> Self {
        u.0
    }
}

impl ::serde::Serialize for UpdateSerializer {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut tup = serializer.serialize_tuple(3)?;
        match &self.0 {
            program::Update::Insert { relid, v } => {
                tup.serialize_element(&true)?;
                tup.serialize_element(relid)?;
                tup.serialize_element(v)?;
            }
            program::Update::DeleteValue { relid, v } => {
                tup.serialize_element(&false)?;
                tup.serialize_element(relid)?;
                tup.serialize_element(v)?;
            }
            _ => panic!("Cannot serialize InsertOrUpdate/Modify/DeleteKey update"),
        };
        tup.end()
    }
}

#[macro_export]
macro_rules! decl_update_deserializer {
    ( $n:ty, $(($rel:expr, $typ:ty)),* ) => {
        impl<'de> ::serde::Deserialize<'de> for $n {
            fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> ::std::result::Result<Self, D::Error> {
                struct UpdateVisitor;

                impl<'de> ::serde::de::Visitor<'de> for UpdateVisitor {
                    type Value = $n;

                    fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                        formatter.write_str("(polarity, relid, value) tuple")
                    }

                    fn visit_seq<A>(self, mut seq: A) -> ::std::result::Result<Self::Value, A::Error>
                    where
                        A: ::serde::de::SeqAccess<'de>,
                    {
                        let polarity = seq.next_element::<bool>()?.ok_or_else(|| <A::Error as ::serde::de::Error>::custom("Missing polarity"))?;
                        let relid = seq.next_element::<::differential_datalog::program::RelId>()?.ok_or_else(|| <A::Error as ::serde::de::Error>::custom("Missing relation id"))?;
                        match relid {
                            $(
                                $rel => {
                                    let v = seq.next_element::<$typ>()?.ok_or_else(|| <A::Error as ::serde::de::Error>::custom("Missing value"))?.into_ddvalue();
                                    if polarity {
                                        Ok(UpdateSerializer(::differential_datalog::program::Update::Insert { relid, v }))
                                    } else {
                                        Ok(UpdateSerializer(::differential_datalog::program::Update::DeleteValue { relid, v }))
                                    }
                                },
                            )*
                            _ => {
                                ::std::result::Result::Err(<A::Error as ::serde::de::Error>::custom(format!("Unknown input relation id {}", relid)))
                            }
                        }
                    }
                }

                deserializer.deserialize_tuple(3, UpdateVisitor)
            }
        }
    };
}

/* FlatBuffers bindings generated by `ddlog` */
#[cfg(feature = "flatbuf")]
pub mod flatbuf;

#[cfg(feature = "flatbuf")]
pub mod flatbuf_generated;

impl TryFrom<&RelIdentifier> for Relations {
    type Error = ();

    fn try_from(rel_id: &RelIdentifier) -> ::std::result::Result<Self, ()> {
        match rel_id {
            RelIdentifier::RelName(rname) => Relations::try_from(rname.as_ref()),
            RelIdentifier::RelId(id) => Relations::try_from(*id),
        }
    }
}

// Macro used to implement `trait D3log`. Invoked from generated code.
#[macro_export]
macro_rules! impl_trait_d3log {
    () => {
        fn d3log_localize_val(
            _relid: ::differential_datalog::program::RelId,
            value: ::differential_datalog::ddval::DDValue,
        ) -> ::core::result::Result<
            (
                ::core::option::Option<::differential_datalog::D3logLocationId>,
                ::differential_datalog::program::RelId,
                ::differential_datalog::ddval::DDValue,
            ),
            ::differential_datalog::ddval::DDValue,
        > {
            ::core::result::Result::Err(value)
        }
    };

    ( $(($out_rel:expr, $in_rel:expr, $typ:ty)),+ ) => {
        pub static D3LOG_CONVERTER_MAP: ::once_cell::sync::Lazy<::std::collections::HashMap<program::RelId, fn(DDValue)->Result<(Option<D3logLocationId>, program::RelId, DDValue), DDValue>>> = ::once_cell::sync::Lazy::new(|| {
            let mut m = ::std::collections::HashMap::new();
            $(
                m.insert($out_rel, { fn __f(val: DDValue) -> Result<(Option<D3logLocationId>, program::RelId, DDValue), DDValue> {
                    if let Some(::ddlog_std::tuple2(loc_id, inner_val)) = <::ddlog_std::tuple2<ddlog_std::Option<D3logLocationId>, $typ>>::try_from_ddvalue_ref(&val) {
                        Ok((::ddlog_std::std2option(*loc_id), $in_rel, (*inner_val).clone().into_ddvalue()))
                    } else {
                        Err(val)
                    }
                } __f as fn(DDValue)->Result<(Option<D3logLocationId>, program::RelId, DDValue), DDValue>});
            )*
            m
        });
        fn d3log_localize_val(relid: program::RelId, val: DDValue) -> Result<(Option<D3logLocationId>, program::RelId, DDValue), DDValue> {
            if let Some(f) = D3LOG_CONVERTER_MAP.get(&relid) {
                f(val)
            } else {
                Err(val)
            }
        }
    };
}

static RAW_RELATION_ID_MAP: ::once_cell::sync::Lazy<
    ::fnv::FnvHashMap<::differential_datalog::program::RelId, &'static ::core::primitive::str>,
> = ::once_cell::sync::Lazy::new(|| {
    let mut map = ::fnv::FnvHashMap::with_capacity_and_hasher(
        crate::RELIDMAP.len(),
        ::fnv::FnvBuildHasher::default(),
    );

    for (&relation, &name) in crate::RELIDMAP.iter() {
        map.insert(relation as ::differential_datalog::program::RelId, name);
    }

    map
});

static RAW_INPUT_RELATION_ID_MAP: ::once_cell::sync::Lazy<
    ::fnv::FnvHashMap<::differential_datalog::program::RelId, &'static ::core::primitive::str>,
> = ::once_cell::sync::Lazy::new(|| {
    let mut map = ::fnv::FnvHashMap::with_capacity_and_hasher(
        crate::INPUT_RELIDMAP.len(),
        ::fnv::FnvBuildHasher::default(),
    );

    for (&relation, &name) in crate::INPUT_RELIDMAP.iter() {
        map.insert(relation as ::differential_datalog::program::RelId, name);
    }

    map
});

/// Create an instance of the DDlog program.
///
/// `config` - program configuration.
/// `do_store` - indicates whether DDlog will track the complete snapshot
///   of output relations.  Should only be used for debugging in order to dump
///   the contents of output tables using `HDDlog::dump_table()`.  Otherwise,
///   indexes are the preferred way to achieve this.
///
/// Returns a handle to the program and initial contents of output relations.
pub fn run_with_config(
    config: ::differential_datalog::program::config::Config,
    do_store: bool,
) -> Result<
    (
        ::differential_datalog::api::HDDlog,
        ::differential_datalog::DeltaMap<DDValue>,
    ),
    String,
> {
    #[cfg(feature = "flatbuf")]
    let flatbuf_converter = Box::new(crate::flatbuf::DDlogFlatbufConverter);
    #[cfg(not(feature = "flatbuf"))]
    let flatbuf_converter = Box::new(differential_datalog::flatbuf::UnimplementedFlatbufConverter);

    ::differential_datalog::api::HDDlog::new(
        config,
        do_store,
        None,
        crate::prog,
        Box::new(crate::Inventory),
        Box::new(crate::D3logInventory),
        flatbuf_converter,
    )
}

/// Create an instance of the DDlog program with default configuration.
///
/// `workers` - number of worker threads (typical values are in the range from 1 to 4).
/// `do_store` - indicates whether DDlog will track the complete snapshot
///   of output relations.  Should only be used for debugging in order to dump
///   the contents of output tables using `HDDlog::dump_table()`.  Otherwise,
///   indexes are the preferred way to achieve this.
///
/// Returns a handle to the program and initial contents of output relations.
pub fn run(
    workers: usize,
    do_store: bool,
) -> Result<
    (
        ::differential_datalog::api::HDDlog,
        ::differential_datalog::DeltaMap<DDValue>,
    ),
    String,
> {
    let config =
        ::differential_datalog::program::config::Config::new().with_timely_workers(workers);

    #[cfg(feature = "flatbuf")]
    let flatbuf_converter = Box::new(crate::flatbuf::DDlogFlatbufConverter);
    #[cfg(not(feature = "flatbuf"))]
    let flatbuf_converter = Box::new(differential_datalog::flatbuf::UnimplementedFlatbufConverter);

    ::differential_datalog::api::HDDlog::new(
        config,
        do_store,
        None,
        crate::prog,
        Box::new(crate::Inventory),
        Box::new(crate::D3logInventory),
        flatbuf_converter,
    )
}

#[no_mangle]
#[cfg(feature = "c_api")]
pub unsafe extern "C" fn ddlog_run_with_config(
    config: *const ::differential_datalog::api::ddlog_config,
    do_store: bool,
    print_err: Option<extern "C" fn(msg: *const ::std::os::raw::c_char)>,
    init_state: *mut *mut ::differential_datalog::DeltaMap<DDValue>,
) -> *const ::differential_datalog::api::HDDlog {
    use ::core::{
        ptr,
        result::Result::{Err, Ok},
    };
    use ::differential_datalog::api::HDDlog;
    use ::std::{boxed::Box, format};
    use ::triomphe::Arc;

    let config = match config.as_ref() {
        None => Default::default(),
        Some(config) => match config.to_rust_api() {
            Ok(cfg) => cfg,
            Err(err) => {
                HDDlog::print_err(
                    print_err,
                    &format!("ddlog_run_with_config(): invalid config: {}", err),
                );
                return ptr::null();
            }
        },
    };

    #[cfg(feature = "flatbuf")]
    let flatbuf_converter = Box::new(crate::flatbuf::DDlogFlatbufConverter);
    #[cfg(not(feature = "flatbuf"))]
    let flatbuf_converter = Box::new(differential_datalog::flatbuf::UnimplementedFlatbufConverter);

    let result = HDDlog::new(
        config,
        do_store,
        print_err,
        crate::prog,
        Box::new(crate::Inventory),
        Box::new(crate::D3logInventory),
        flatbuf_converter,
    );

    match result {
        Ok((hddlog, init)) => {
            if !init_state.is_null() {
                *init_state = Box::into_raw(Box::new(init));
            }
            // Note: This is `triomphe::Arc`, *not* `std::sync::Arc`
            Arc::into_raw(Arc::new(hddlog))
        }
        Err(err) => {
            HDDlog::print_err(print_err, &format!("HDDlog::new() failed: {}", err));
            ptr::null()
        }
    }
}

#[no_mangle]
#[cfg(feature = "c_api")]
pub unsafe extern "C" fn ddlog_run(
    workers: ::std::os::raw::c_uint,
    do_store: bool,
    print_err: Option<extern "C" fn(msg: *const ::std::os::raw::c_char)>,
    init_state: *mut *mut ::differential_datalog::DeltaMap<DDValue>,
) -> *const ::differential_datalog::api::HDDlog {
    let config = ::differential_datalog::api::ddlog_config {
        num_timely_workers: workers,
        ..Default::default()
    };
    ddlog_run_with_config(&config, do_store, print_err, init_state)
}


pub mod typedefs
{
    pub use ::types::Edge;
    pub use ::types::src;
    pub use ::types::dest;
    pub mod ddlog_std
    {
        pub use ::ddlog_std::s8;
        pub use ::ddlog_std::s64;
        pub use ::ddlog_std::s32;
        pub use ::ddlog_std::s16;
        pub use ::ddlog_std::s128;
        pub use ::ddlog_std::Vec;
        pub use ::ddlog_std::Singleton;
        pub use ::ddlog_std::Set;
        pub use ::ddlog_std::Result;
        pub use ::ddlog_std::Ref;
        pub use ::ddlog_std::Option;
        pub use ::ddlog_std::Map;
        pub use ::ddlog_std::Group;
        pub use ::ddlog_std::Either;
        pub use ::ddlog_std::DDlogGroup;
        pub use ::ddlog_std::DDWeight;
        pub use ::ddlog_std::DDNestedTS;
        pub use ::ddlog_std::DDIteration;
        pub use ::ddlog_std::DDEpoch;
        pub use ::ddlog_std::D3logLocationId;
        pub use ::ddlog_std::zip;
        pub use ::ddlog_std::vec_zip;
        pub use ::ddlog_std::vec_with_length;
        pub use ::ddlog_std::vec_with_capacity;
        pub use ::ddlog_std::vec_update_nth;
        pub use ::ddlog_std::vec_truncate;
        pub use ::ddlog_std::vec_to_set;
        pub use ::ddlog_std::vec_swap_nth;
        pub use ::ddlog_std::vec_sort_imm;
        pub use ::ddlog_std::vec_sort;
        pub use ::ddlog_std::vec_singleton;
        pub use ::ddlog_std::vec_reverse;
        pub use ::ddlog_std::vec_resize;
        pub use ::ddlog_std::vec_push_imm;
        pub use ::ddlog_std::vec_push;
        pub use ::ddlog_std::vec_pop;
        pub use ::ddlog_std::vec_nth;
        pub use ::ddlog_std::vec_len;
        pub use ::ddlog_std::vec_is_empty;
        pub use ::ddlog_std::vec_empty;
        pub use ::ddlog_std::vec_contains;
        pub use ::ddlog_std::vec_append;
        pub use ::ddlog_std::values;
        pub use ::ddlog_std::update_nth;
        pub use ::ddlog_std::unzip;
        pub use ::ddlog_std::unwrap_or_default_ddlog_std_Result__V_E_V;
        pub use ::ddlog_std::unwrap_or_default_ddlog_std_Option__A_A;
        pub use ::ddlog_std::unwrap_or_ddlog_std_Result__V_E_V_V;
        pub use ::ddlog_std::unwrap_or_ddlog_std_Option__A_A_A;
        pub use ::ddlog_std::unions;
        pub use ::ddlog_std::union_ddlog_std_Vec__ddlog_std_Set__X_ddlog_std_Set__X;
        pub use ::ddlog_std::union_ddlog_std_Set__X_ddlog_std_Set__X_ddlog_std_Set__X;
        pub use ::ddlog_std::union_ddlog_std_Map__K_V_ddlog_std_Map__K_V_ddlog_std_Map__K_V;
        pub use ::ddlog_std::union_ddlog_std_Group__K_ddlog_std_Ref__ddlog_std_Set__A_ddlog_std_Ref__ddlog_std_Set__A;
        pub use ::ddlog_std::union_ddlog_std_Group__K_ddlog_std_Set__A_ddlog_std_Set__A;
        pub use ::ddlog_std::u8_pow32;
        pub use ::ddlog_std::u64_pow32;
        pub use ::ddlog_std::u32_pow32;
        pub use ::ddlog_std::u16_pow32;
        pub use ::ddlog_std::u128_pow32;
        pub use ::ddlog_std::truncate;
        pub use ::ddlog_std::trim;
        pub use ::ddlog_std::to_vec_ddlog_std_Set__A_ddlog_std_Vec__A;
        pub use ::ddlog_std::to_vec_ddlog_std_Group__K_V_ddlog_std_Vec__V;
        pub use ::ddlog_std::to_vec_ddlog_std_Option__X_ddlog_std_Vec__X;
        pub use ::ddlog_std::to_uppercase;
        pub use ::ddlog_std::to_string_debug;
        pub use ::ddlog_std::to_string___Stringval___Stringval;
        pub use ::ddlog_std::to_string___Bitval128___Stringval;
        pub use ::ddlog_std::to_string___Bitval64___Stringval;
        pub use ::ddlog_std::to_string___Bitval32___Stringval;
        pub use ::ddlog_std::to_string___Bitval16___Stringval;
        pub use ::ddlog_std::to_string___Bitval8___Stringval;
        pub use ::ddlog_std::to_string___Signedval128___Stringval;
        pub use ::ddlog_std::to_string___Signedval64___Stringval;
        pub use ::ddlog_std::to_string___Signedval32___Stringval;
        pub use ::ddlog_std::to_string___Signedval16___Stringval;
        pub use ::ddlog_std::to_string___Signedval8___Stringval;
        pub use ::ddlog_std::to_string___Doubleval___Stringval;
        pub use ::ddlog_std::to_string___Floatval___Stringval;
        pub use ::ddlog_std::to_string___Intval___Stringval;
        pub use ::ddlog_std::to_string___Boolval___Stringval;
        pub use ::ddlog_std::to_string_ddlog_std_DDNestedTS___Stringval;
        pub use ::ddlog_std::to_setmap;
        pub use ::ddlog_std::to_set_ddlog_std_Vec__A_ddlog_std_Set__A;
        pub use ::ddlog_std::to_set_ddlog_std_Group__K_V_ddlog_std_Set__V;
        pub use ::ddlog_std::to_set_ddlog_std_Option__X_ddlog_std_Set__X;
        pub use ::ddlog_std::to_map_ddlog_std_Vec____Tuple2__K_V_ddlog_std_Map__K_V;
        pub use ::ddlog_std::to_map_ddlog_std_Group__K1___Tuple2__K2_V_ddlog_std_Map__K2_V;
        pub use ::ddlog_std::to_lowercase;
        pub use ::ddlog_std::to_bytes;
        pub use ::ddlog_std::swap_nth;
        pub use ::ddlog_std::substr;
        pub use ::ddlog_std::string_trim;
        pub use ::ddlog_std::string_to_uppercase;
        pub use ::ddlog_std::string_to_lowercase;
        pub use ::ddlog_std::string_to_bytes;
        pub use ::ddlog_std::string_substr;
        pub use ::ddlog_std::string_starts_with;
        pub use ::ddlog_std::string_split;
        pub use ::ddlog_std::string_reverse;
        pub use ::ddlog_std::string_replace;
        pub use ::ddlog_std::string_len;
        pub use ::ddlog_std::string_join;
        pub use ::ddlog_std::string_ends_with;
        pub use ::ddlog_std::string_contains;
        pub use ::ddlog_std::str_to_lower;
        pub use ::ddlog_std::starts_with;
        pub use ::ddlog_std::split;
        pub use ::ddlog_std::sort_imm;
        pub use ::ddlog_std::sort;
        pub use ::ddlog_std::size_ddlog_std_Set__X___Bitval64;
        pub use ::ddlog_std::size_ddlog_std_Map__K_V___Bitval64;
        pub use ::ddlog_std::size_ddlog_std_Group__K_V___Bitval64;
        pub use ::ddlog_std::setref_unions;
        pub use ::ddlog_std::set_unions;
        pub use ::ddlog_std::set_union;
        pub use ::ddlog_std::set_to_vec;
        pub use ::ddlog_std::set_size;
        pub use ::ddlog_std::set_singleton;
        pub use ::ddlog_std::set_nth;
        pub use ::ddlog_std::set_is_empty;
        pub use ::ddlog_std::set_intersection;
        pub use ::ddlog_std::set_insert_imm;
        pub use ::ddlog_std::set_insert;
        pub use ::ddlog_std::set_empty;
        pub use ::ddlog_std::set_difference;
        pub use ::ddlog_std::set_contains;
        pub use ::ddlog_std::s8_pow32;
        pub use ::ddlog_std::s64_pow32;
        pub use ::ddlog_std::s32_pow32;
        pub use ::ddlog_std::s16_pow32;
        pub use ::ddlog_std::s128_pow32;
        pub use ::ddlog_std::reverse_imm;
        pub use ::ddlog_std::reverse_ddlog_std_Vec__X___Tuple0__;
        pub use ::ddlog_std::reverse___Stringval___Stringval;
        pub use ::ddlog_std::result_unwrap_or_default;
        pub use ::ddlog_std::resize;
        pub use ::ddlog_std::replace;
        pub use ::ddlog_std::remove;
        pub use ::ddlog_std::ref_new;
        pub use ::ddlog_std::range_vec;
        pub use ::ddlog_std::push_imm;
        pub use ::ddlog_std::push;
        pub use ::ddlog_std::pow32___Intval___Bitval32___Intval;
        pub use ::ddlog_std::pow32___Signedval128___Bitval32___Signedval128;
        pub use ::ddlog_std::pow32___Signedval64___Bitval32___Signedval64;
        pub use ::ddlog_std::pow32___Signedval32___Bitval32___Signedval32;
        pub use ::ddlog_std::pow32___Signedval16___Bitval32___Signedval16;
        pub use ::ddlog_std::pow32___Signedval8___Bitval32___Signedval8;
        pub use ::ddlog_std::pow32___Bitval128___Bitval32___Bitval128;
        pub use ::ddlog_std::pow32___Bitval64___Bitval32___Bitval64;
        pub use ::ddlog_std::pow32___Bitval32___Bitval32___Bitval32;
        pub use ::ddlog_std::pow32___Bitval16___Bitval32___Bitval16;
        pub use ::ddlog_std::pow32___Bitval8___Bitval32___Bitval8;
        pub use ::ddlog_std::pop;
        pub use ::ddlog_std::parse_dec_u64;
        pub use ::ddlog_std::parse_dec_i64;
        pub use ::ddlog_std::option_unwrap_or_default;
        pub use ::ddlog_std::ok_or_else;
        pub use ::ddlog_std::ok_or;
        pub use ::ddlog_std::ntohs;
        pub use ::ddlog_std::ntohl;
        pub use ::ddlog_std::nth_value;
        pub use ::ddlog_std::nth_key;
        pub use ::ddlog_std::nth_ddlog_std_Set__X___Bitval64_ddlog_std_Option__X;
        pub use ::ddlog_std::nth_ddlog_std_Vec__X___Bitval64_ddlog_std_Option__X;
        pub use ::ddlog_std::nth_ddlog_std_Group__K_V___Bitval64_ddlog_std_Option__V;
        pub use ::ddlog_std::min_ddlog_std_Group__K_V_V;
        pub use ::ddlog_std::min_A_A_A;
        pub use ::ddlog_std::max_ddlog_std_Group__K_V_V;
        pub use ::ddlog_std::max_A_A_A;
        pub use ::ddlog_std::map_values;
        pub use ::ddlog_std::map_union;
        pub use ::ddlog_std::map_size;
        pub use ::ddlog_std::map_singleton;
        pub use ::ddlog_std::map_remove;
        pub use ::ddlog_std::map_nth_value;
        pub use ::ddlog_std::map_nth_key;
        pub use ::ddlog_std::map_keys;
        pub use ::ddlog_std::map_is_empty;
        pub use ::ddlog_std::map_insert_imm;
        pub use ::ddlog_std::map_insert;
        pub use ::ddlog_std::map_get;
        pub use ::ddlog_std::map_err;
        pub use ::ddlog_std::map_empty;
        pub use ::ddlog_std::map_contains_key;
        pub use ::ddlog_std::map_ddlog_std_Result__V1_E___Closureimm_V1_ret_V2_ddlog_std_Result__V2_E;
        pub use ::ddlog_std::map_ddlog_std_Option__A___Closureimm_A_ret_B_ddlog_std_Option__B;
        pub use ::ddlog_std::len_ddlog_std_Vec__X___Bitval64;
        pub use ::ddlog_std::len___Stringval___Bitval64;
        pub use ::ddlog_std::keys;
        pub use ::ddlog_std::key;
        pub use ::ddlog_std::join;
        pub use ::ddlog_std::is_some;
        pub use ::ddlog_std::is_ok;
        pub use ::ddlog_std::is_none;
        pub use ::ddlog_std::is_err;
        pub use ::ddlog_std::is_empty_ddlog_std_Set__X___Boolval;
        pub use ::ddlog_std::is_empty_ddlog_std_Map__K_V___Boolval;
        pub use ::ddlog_std::is_empty_ddlog_std_Vec__X___Boolval;
        pub use ::ddlog_std::intersection;
        pub use ::ddlog_std::insert_imm_ddlog_std_Set__X_X_ddlog_std_Set__X;
        pub use ::ddlog_std::insert_imm_ddlog_std_Map__K_V_K_V_ddlog_std_Map__K_V;
        pub use ::ddlog_std::insert_ddlog_std_Set__X_X___Tuple0__;
        pub use ::ddlog_std::insert_ddlog_std_Map__K_V_K_V___Tuple0__;
        pub use ::ddlog_std::htons;
        pub use ::ddlog_std::htonl;
        pub use ::ddlog_std::hex;
        pub use ::ddlog_std::hash64;
        pub use ::ddlog_std::hash32;
        pub use ::ddlog_std::hash128;
        pub use ::ddlog_std::group_unzip;
        pub use ::ddlog_std::group_to_vec;
        pub use ::ddlog_std::group_to_setmap;
        pub use ::ddlog_std::group_to_set;
        pub use ::ddlog_std::group_to_map;
        pub use ::ddlog_std::group_sum;
        pub use ::ddlog_std::group_setref_unions;
        pub use ::ddlog_std::group_set_unions;
        pub use ::ddlog_std::group_nth;
        pub use ::ddlog_std::group_min;
        pub use ::ddlog_std::group_max;
        pub use ::ddlog_std::group_key;
        pub use ::ddlog_std::group_first;
        pub use ::ddlog_std::group_count;
        pub use ::ddlog_std::get;
        pub use ::ddlog_std::from_utf8;
        pub use ::ddlog_std::from_utf16;
        pub use ::ddlog_std::first;
        pub use ::ddlog_std::ends_with;
        pub use ::ddlog_std::encode_utf16;
        pub use ::ddlog_std::difference;
        pub use ::ddlog_std::deref;
        pub use ::ddlog_std::default;
        pub use ::ddlog_std::count;
        pub use ::ddlog_std::contains_key;
        pub use ::ddlog_std::contains_ddlog_std_Set__X_X___Boolval;
        pub use ::ddlog_std::contains_ddlog_std_Vec__X_X___Boolval;
        pub use ::ddlog_std::contains___Stringval___Stringval___Boolval;
        pub use ::ddlog_std::bigint_pow32;
        pub use ::ddlog_std::append;
        pub use ::ddlog_std::and_then;
        pub use ::ddlog_std::__builtin_2string;
    }
    pub mod debug
    {
        pub use ::debug::DDlogOpId;
        pub use ::debug::debug_split_group;
        pub use ::debug::debug_event_join;
        pub use ::debug::debug_event;
    }
    pub mod internment
    {
        pub use ::internment::istring;
        pub use ::internment::Intern;
        pub use ::internment::trim;
        pub use ::internment::to_uppercase;
        pub use ::internment::to_string;
        pub use ::internment::to_lowercase;
        pub use ::internment::to_bytes;
        pub use ::internment::substr;
        pub use ::internment::starts_with;
        pub use ::internment::split;
        pub use ::internment::reverse;
        pub use ::internment::replace;
        pub use ::internment::parse_dec_u64;
        pub use ::internment::parse_dec_i64;
        pub use ::internment::len;
        pub use ::internment::join;
        pub use ::internment::ival;
        pub use ::internment::istring_trim;
        pub use ::internment::istring_to_uppercase;
        pub use ::internment::istring_to_lowercase;
        pub use ::internment::istring_to_bytes;
        pub use ::internment::istring_substr;
        pub use ::internment::istring_starts_with;
        pub use ::internment::istring_split;
        pub use ::internment::istring_reverse;
        pub use ::internment::istring_replace;
        pub use ::internment::istring_len;
        pub use ::internment::istring_join;
        pub use ::internment::istring_ends_with;
        pub use ::internment::istring_contains;
        pub use ::internment::intern;
        pub use ::internment::ends_with;
        pub use ::internment::contains;
    }
}
decl_update_deserializer!(UpdateSerializer,(0, types::Edge), (1, ddlog_std::tuple2<u32, u32>));
impl TryFrom<&str> for Relations {
    type Error = ();
    fn try_from(rname: &str) -> ::std::result::Result<Self, ()> {
         match rname {
        "Edge" => Ok(Relations::Edge),
        "StronglyConnected" => Ok(Relations::StronglyConnected),
        "__Null" => Ok(Relations::__Null),
        "ddlog_std::Singleton" => Ok(Relations::ddlog_std_Singleton),
             _  => Err(()),
         }
    }
}
impl Relations {
    pub fn is_output(&self) -> bool {
        match self {
        Relations::StronglyConnected => true,
            _  => false
        }
    }
}
impl Relations {
    pub fn is_input(&self) -> bool {
        match self {
        Relations::Edge => true,
            _  => false
        }
    }
}
impl Relations {
    pub fn type_id(&self) -> ::std::any::TypeId {
        match self {
            Relations::Edge => ::std::any::TypeId::of::<types::Edge>(),
            Relations::StronglyConnected => ::std::any::TypeId::of::<ddlog_std::tuple2<u32, u32>>(),
            Relations::__Null => ::std::any::TypeId::of::<()>(),
            Relations::ddlog_std_Singleton => ::std::any::TypeId::of::<ddlog_std::Singleton>(),
        }
    }
}
impl TryFrom<program::RelId> for Relations {
    type Error = ();
    fn try_from(rid: program::RelId) -> ::std::result::Result<Self, ()> {
         match rid {
        0 => Ok(Relations::Edge),
        1 => Ok(Relations::StronglyConnected),
        2 => Ok(Relations::__Null),
        3 => Ok(Relations::ddlog_std_Singleton),
             _  => Err(())
         }
    }
}
pub fn relid2name(rid: program::RelId) -> ::std::option::Option<&'static str> {
   match rid {
        0 => ::core::option::Option::Some("Edge"),
        1 => ::core::option::Option::Some("StronglyConnected"),
        2 => ::core::option::Option::Some("__Null"),
        3 => ::core::option::Option::Some("ddlog_std::Singleton"),
       _  => None
   }
}
#[cfg(feature = "c_api")]
pub fn relid2cname(rid: program::RelId) -> ::std::option::Option<&'static ::std::ffi::CStr> {
    RELIDMAPC.get(&rid).copied()
}
pub fn rel_name2orig_name(rname: &str) -> ::std::option::Option<&'static str> {
   match rname {
        "Edge" => ::core::option::Option::Some("Edge"),
        "StronglyConnected" => ::core::option::Option::Some("StronglyConnected"),
        "__Null" => ::core::option::Option::Some("__Null"),
        "ddlog_std::Singleton" => ::core::option::Option::Some("ddlog_std::Singleton"),
       _  => None
   }
}
#[cfg(feature = "c_api")]
pub fn rel_name2orig_cname(rname: &str) -> ::std::option::Option<&'static ::std::ffi::CStr> {
   match rname {
        "Edge" => Some(::std::ffi::CStr::from_bytes_with_nul(b"Edge\0") .expect("Unreachable: A null byte was specifically inserted")),
        "StronglyConnected" => Some(::std::ffi::CStr::from_bytes_with_nul(b"StronglyConnected\0") .expect("Unreachable: A null byte was specifically inserted")),
        "__Null" => Some(::std::ffi::CStr::from_bytes_with_nul(b"__Null\0") .expect("Unreachable: A null byte was specifically inserted")),
        "ddlog_std::Singleton" => Some(::std::ffi::CStr::from_bytes_with_nul(b"ddlog_std::Singleton\0") .expect("Unreachable: A null byte was specifically inserted")),
       _  => None
   }
}
pub fn orig_rel_name2name(rname: &str) -> ::std::option::Option<&'static str> {
   match rname {
        "Edge" => ::core::option::Option::Some("Edge"),
        "StronglyConnected" => ::core::option::Option::Some("StronglyConnected"),
        "__Null" => ::core::option::Option::Some("__Null"),
        "ddlog_std::Singleton" => ::core::option::Option::Some("ddlog_std::Singleton"),
       _  => None
   }
}
/// A map of `RelId`s to their name as an `&'static str`
pub static RELIDMAP: ::once_cell::sync::Lazy<::fnv::FnvHashMap<Relations, &'static str>> =
    ::once_cell::sync::Lazy::new(|| {
        let mut map = ::fnv::FnvHashMap::with_capacity_and_hasher(4, ::fnv::FnvBuildHasher::default());
        map.insert(Relations::Edge, "Edge");
        map.insert(Relations::StronglyConnected, "StronglyConnected");
        map.insert(Relations::__Null, "__Null");
        map.insert(Relations::ddlog_std_Singleton, "ddlog_std::Singleton");
        map
    });
/// A map of `RelId`s to their name as an `&'static CStr`
#[cfg(feature = "c_api")]
pub static RELIDMAPC: ::once_cell::sync::Lazy<::fnv::FnvHashMap<program::RelId, &'static ::std::ffi::CStr>> =
    ::once_cell::sync::Lazy::new(|| {
        let mut map = ::fnv::FnvHashMap::with_capacity_and_hasher(4, ::fnv::FnvBuildHasher::default());
        map.insert(0, ::std::ffi::CStr::from_bytes_with_nul(b"Edge\0").expect("Unreachable: A null byte was specifically inserted"));
        map.insert(1, ::std::ffi::CStr::from_bytes_with_nul(b"StronglyConnected\0").expect("Unreachable: A null byte was specifically inserted"));
        map.insert(2, ::std::ffi::CStr::from_bytes_with_nul(b"__Null\0").expect("Unreachable: A null byte was specifically inserted"));
        map.insert(3, ::std::ffi::CStr::from_bytes_with_nul(b"ddlog_std::Singleton\0").expect("Unreachable: A null byte was specifically inserted"));
        map
    });
/// A map of input `Relations`s to their name as an `&'static str`
pub static INPUT_RELIDMAP: ::once_cell::sync::Lazy<::fnv::FnvHashMap<Relations, &'static str>> =
    ::once_cell::sync::Lazy::new(|| {
        let mut map = ::fnv::FnvHashMap::with_capacity_and_hasher(1, ::fnv::FnvBuildHasher::default());
        map.insert(Relations::Edge, "Edge");
        map
    });
/// A map of output `Relations`s to their name as an `&'static str`
pub static OUTPUT_RELIDMAP: ::once_cell::sync::Lazy<::fnv::FnvHashMap<Relations, &'static str>> =
    ::once_cell::sync::Lazy::new(|| {
        let mut map = ::fnv::FnvHashMap::with_capacity_and_hasher(1, ::fnv::FnvBuildHasher::default());
        map.insert(Relations::StronglyConnected, "StronglyConnected");
        map
    });
impl TryFrom<&str> for Indexes {
    type Error = ();
    fn try_from(iname: &str) -> ::std::result::Result<Self, ()> {
         match iname {
        "__Null_by_none" => Ok(Indexes::__Null_by_none),
             _  => Err(())
         }
    }
}
impl TryFrom<program::IdxId> for Indexes {
    type Error = ();
    fn try_from(iid: program::IdxId) -> ::core::result::Result<Self, ()> {
         match iid {
        0 => Ok(Indexes::__Null_by_none),
             _  => Err(())
         }
    }
}
pub fn indexid2name(iid: program::IdxId) -> ::std::option::Option<&'static str> {
   match iid {
        0 => ::core::option::Option::Some("__Null_by_none"),
       _  => None
   }
}
#[cfg(feature = "c_api")]
pub fn indexid2cname(iid: program::IdxId) -> ::std::option::Option<&'static ::std::ffi::CStr> {
    IDXIDMAPC.get(&iid).copied()
}
/// A map of `Indexes` to their name as an `&'static str`
pub static IDXIDMAP: ::once_cell::sync::Lazy<::fnv::FnvHashMap<Indexes, &'static str>> =
    ::once_cell::sync::Lazy::new(|| {
        let mut map = ::fnv::FnvHashMap::with_capacity_and_hasher(1, ::fnv::FnvBuildHasher::default());
        map.insert(Indexes::__Null_by_none, "__Null_by_none");
        map
    });
/// A map of `IdxId`s to their name as an `&'static CStr`
#[cfg(feature = "c_api")]
pub static IDXIDMAPC: ::once_cell::sync::Lazy<::fnv::FnvHashMap<program::IdxId, &'static ::std::ffi::CStr>> =
    ::once_cell::sync::Lazy::new(|| {
        let mut map = ::fnv::FnvHashMap::with_capacity_and_hasher(1, ::fnv::FnvBuildHasher::default());
        map.insert(0, ::std::ffi::CStr::from_bytes_with_nul(b"__Null_by_none\0").expect("Unreachable: A null byte was specifically inserted"));
        map
    });
pub fn relval_from_record(relation: Relations, record: &::differential_datalog::record::Record) -> ::std::result::Result<DDValue, ::std::string::String> {
    match relation {
        Relations::Edge => {
            Ok(<types::Edge as ::differential_datalog::record::FromRecord>::from_record(record)?.into_ddvalue())
        },
        Relations::StronglyConnected => {
            Ok(<ddlog_std::tuple2<u32, u32> as ::differential_datalog::record::FromRecord>::from_record(record)?.into_ddvalue())
        },
        Relations::__Null => {
            Ok(<() as ::differential_datalog::record::FromRecord>::from_record(record)?.into_ddvalue())
        },
        Relations::ddlog_std_Singleton => {
            Ok(<ddlog_std::Singleton as ::differential_datalog::record::FromRecord>::from_record(record)?.into_ddvalue())
        }
    }
}
pub fn relkey_from_record(relation: Relations, record: &::differential_datalog::record::Record) -> ::std::result::Result<DDValue, ::std::string::String> {
    match relation {
        _ => Err(format!("relation {:?} does not have a primary key", relation)),
    }
}
pub fn idxkey_from_record(idx: Indexes, record: &::differential_datalog::record::Record) -> ::std::result::Result<DDValue, ::std::string::String> {
    match idx {
        Indexes::__Null_by_none => {
            Ok(<() as ::differential_datalog::record::FromRecord>::from_record(record)?.into_ddvalue())
        }
    }
}
pub fn indexes2arrid(idx: Indexes) -> program::ArrId {
    match idx {
        Indexes::__Null_by_none => ( 2, 0),
    }
}
#[derive(Copy,Clone,Debug,PartialEq,Eq,Hash)]
pub enum Relations {
    Edge = 0,
    StronglyConnected = 1,
    __Null = 2,
    ddlog_std_Singleton = 3
}
#[derive(Copy,Clone,Debug,PartialEq,Eq,Hash)]
pub enum Indexes {
    __Null_by_none = 0
}
impl_trait_d3log!();
pub fn prog(__update_cb: std::sync::Arc<dyn program::RelationCallback>) -> program::Program {
    let Edge = ::differential_datalog::program::Relation {
        name: ::std::borrow::Cow::from("Edge @ scc_transformer.dl:3:1"),
        input: true,
        distinct: false,
        caching_mode: ::differential_datalog::program::CachingMode::Set,
        key_func: ::core::option::Option::None,
        id: 0,
        rules: vec![
        ],
        arrangements: vec![
        ],
        change_cb: ::core::option::Option::None,
    };
    let StronglyConnected = ::differential_datalog::program::Relation {
        name: ::std::borrow::Cow::from("StronglyConnected @ scc_transformer.dl:13:1"),
        input: false,
        distinct: true,
        caching_mode: ::differential_datalog::program::CachingMode::Set,
        key_func: ::core::option::Option::None,
        id: 1,
        rules: vec![
        ],
        arrangements: vec![
        ],
        change_cb: ::core::option::Option::Some(::std::sync::Arc::clone(&__update_cb)),
    };
    let __Null = ::differential_datalog::program::Relation {
        name: ::std::borrow::Cow::from("__Null @ :1:1"),
        input: false,
        distinct: false,
        caching_mode: ::differential_datalog::program::CachingMode::Set,
        key_func: ::core::option::Option::None,
        id: 2,
        rules: vec![
        ],
        arrangements: vec![
            types::__Arng___Null_0.clone(),
        ],
        change_cb: ::core::option::Option::None,
    };
    let ddlog_std_Singleton = ::differential_datalog::program::Relation {
        name: ::std::borrow::Cow::from("ddlog_std::Singleton @ G:\\Users\\Chase\\Code\\Rust\\differential-datalog\\lib\\ddlog_std.dl:939:1"),
        input: false,
        distinct: false,
        caching_mode: ::differential_datalog::program::CachingMode::Set,
        key_func: ::core::option::Option::None,
        id: 3,
        rules: vec![
        ],
        arrangements: vec![
        ],
        change_cb: ::core::option::Option::None,
    };
    let nodes: std::vec::Vec<program::ProgNode> = vec![
            program::ProgNode::Rel{rel: Edge},
            program::ProgNode::Apply{tfun: types::__apply_1},
            program::ProgNode::Rel{rel: StronglyConnected},
            program::ProgNode::Rel{rel: __Null},
            program::ProgNode::Rel{rel: ddlog_std_Singleton}
    ];
    let delayed_rels = vec![];
    let init_data: std::vec::Vec<(program::RelId, DDValue)> = vec![ddlog_std::__Fact_ddlog_std_Singleton_0.clone()];
    program::Program {
        nodes,
        delayed_rels,
        init_data,
    }
}
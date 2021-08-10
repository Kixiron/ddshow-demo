#![allow(
    path_statements,
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
    clippy::match_single_binding,
    clippy::ptr_arg,
    clippy::redundant_closure,
    clippy::needless_lifetimes,
    clippy::borrowed_box,
    clippy::map_clone,
    clippy::toplevel_ref_arg,
    clippy::double_parens,
    clippy::collapsible_if,
    clippy::clone_on_copy,
    clippy::unused_unit,
    clippy::deref_addrof,
    clippy::clone_on_copy,
    clippy::needless_return,
    clippy::op_ref,
    clippy::match_like_matches_macro,
    clippy::comparison_chain,
    clippy::len_zero,
    clippy::extra_unused_lifetimes
)]

use ::num::One;
use ::std::ops::Deref;

use ::differential_dataflow::collection;
use ::timely::communication;
use ::timely::dataflow::scopes;
use ::timely::worker;

use ::ddlog_derive::{FromRecord, IntoRecord, Mutator};
use ::differential_datalog::ddval::DDValue;
use ::differential_datalog::ddval::DDValConvert;
use ::differential_datalog::program;
use ::differential_datalog::program::TupleTS;
use ::differential_datalog::program::XFormArrangement;
use ::differential_datalog::program::XFormCollection;
use ::differential_datalog::program::Weight;
use ::differential_datalog::record::FromRecord;
use ::differential_datalog::record::IntoRecord;
use ::differential_datalog::record::Mutator;
use ::serde::Deserialize;
use ::serde::Serialize;


// `usize` and `isize` are builtin Rust types; we therefore declare an alias to DDlog's `usize` and
// `isize`.
pub type std_usize = u64;
pub type std_isize = i64;


#[derive(Eq, Ord, Clone, Hash, PartialEq, PartialOrd, IntoRecord, Mutator, Default, Serialize, Deserialize, FromRecord)]
#[ddlog(rename = "Edge")]
pub struct Edge {
    pub src: u32,
    pub dest: u32
}
impl abomonation::Abomonation for Edge{}
impl ::std::fmt::Display for Edge {
    fn fmt(&self, __formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            Edge{src,dest} => {
                __formatter.write_str("Edge{")?;
                ::std::fmt::Debug::fmt(src, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(dest, __formatter)?;
                __formatter.write_str("}")
            }
        }
    }
}
impl ::std::fmt::Debug for Edge {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self, f)
    }
}
pub fn dest(edge: & Edge) -> u32
{   return edge.dest.clone()
}
pub fn src(edge: & Edge) -> u32
{   return edge.src.clone()
}
pub static __Arng___Null_0 : ::once_cell::sync::Lazy<program::Arrangement> = ::once_cell::sync::Lazy::new(|| {
    program::Arrangement::Map{
       name: std::borrow::Cow::from(r###"_ /*join*/"###),
        afun: {fn __f(__v: DDValue) -> ::std::option::Option<(DDValue,DDValue)>
        {
            let __cloned = __v.clone();
            match unsafe { <()>::from_ddvalue_unchecked(__v) } {
                _ => Some((()).into_ddvalue()),
                _ => None
            }.map(|x|(x,__cloned))
        }
        __f},
        queryable: true
    }
});
pub fn __apply_1 () -> Box<
    dyn for<'a> Fn(
        &mut ::fnv::FnvHashMap<
            program::RelId,
            collection::Collection<
            scopes::Child<'a, worker::Worker<communication::Allocator>, program::TS>,
                DDValue,
                Weight,
            >,
        >,
    ),
> {
    Box::new(|collections| {
        let (StronglyConnected) = types__graph::SCC(collections.get(&(0)).unwrap(), (|__v: DDValue| unsafe {<Edge>::from_ddvalue_unchecked(__v) }), src, dest, (|v| v.into_ddvalue()));
        collections.insert(1, StronglyConnected);
    })
}
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
#[ddlog(rename = "Connected")]
pub struct Connected {
    pub src: u32,
    pub dest: u32
}
impl abomonation::Abomonation for Connected{}
impl ::std::fmt::Display for Connected {
    fn fmt(&self, __formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            Connected{src,dest} => {
                __formatter.write_str("Connected{")?;
                ::std::fmt::Debug::fmt(src, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(dest, __formatter)?;
                __formatter.write_str("}")
            }
        }
    }
}
impl ::std::fmt::Debug for Connected {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self, f)
    }
}
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
#[derive(Eq, Ord, Clone, Hash, PartialEq, PartialOrd, IntoRecord, Mutator, Default, Serialize, Deserialize, FromRecord)]
#[ddlog(rename = "StronglyConnected")]
pub struct StronglyConnected {
    pub node: u32,
    pub regime: u32
}
impl abomonation::Abomonation for StronglyConnected{}
impl ::std::fmt::Display for StronglyConnected {
    fn fmt(&self, __formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            StronglyConnected{node,regime} => {
                __formatter.write_str("StronglyConnected{")?;
                ::std::fmt::Debug::fmt(node, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(regime, __formatter)?;
                __formatter.write_str("}")
            }
        }
    }
}
impl ::std::fmt::Debug for StronglyConnected {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self, f)
    }
}
pub static __Arng_Edge_0 : ::once_cell::sync::Lazy<program::Arrangement> = ::once_cell::sync::Lazy::new(|| {
    program::Arrangement::Map{
       name: std::borrow::Cow::from(r###"(Edge{.src=(_: bit<32>), .dest=(_0: bit<32>)}: Edge) /*join*/"###),
        afun: {fn __f(__v: DDValue) -> ::std::option::Option<(DDValue,DDValue)>
        {
            let __cloned = __v.clone();
            match unsafe { <Edge>::from_ddvalue_unchecked(__v) } {
                Edge{src: _, dest: ref _0} => Some(((*_0).clone()).into_ddvalue()),
                _ => None
            }.map(|x|(x,__cloned))
        }
        __f},
        queryable: false
    }
});
pub static __Arng_Connected_0 : ::once_cell::sync::Lazy<program::Arrangement> = ::once_cell::sync::Lazy::new(|| {
    program::Arrangement::Map{
       name: std::borrow::Cow::from(r###"(Connected{.src=(_0: bit<32>), .dest=(_: bit<32>)}: Connected) /*join*/"###),
        afun: {fn __f(__v: DDValue) -> ::std::option::Option<(DDValue,DDValue)>
        {
            let __cloned = __v.clone();
            match unsafe { <Connected>::from_ddvalue_unchecked(__v) } {
                Connected{src: ref _0, dest: _} => Some(((*_0).clone()).into_ddvalue()),
                _ => None
            }.map(|x|(x,__cloned))
        }
        __f},
        queryable: false
    }
});
pub static __Arng_Connected_1 : ::once_cell::sync::Lazy<program::Arrangement> = ::once_cell::sync::Lazy::new(|| {
    program::Arrangement::Map{
       name: std::borrow::Cow::from(r###"(Connected{.src=(_1: bit<32>), .dest=(_0: bit<32>)}: Connected) /*join*/"###),
        afun: {fn __f(__v: DDValue) -> ::std::option::Option<(DDValue,DDValue)>
        {
            let __cloned = __v.clone();
            match unsafe { <Connected>::from_ddvalue_unchecked(__v) } {
                Connected{src: ref _1, dest: ref _0} => Some((ddlog_std::tuple2((*_0).clone(), (*_1).clone())).into_ddvalue()),
                _ => None
            }.map(|x|(x,__cloned))
        }
        __f},
        queryable: false
    }
});
pub static __Arng_Connected_2 : ::once_cell::sync::Lazy<program::Arrangement> = ::once_cell::sync::Lazy::new(|| {
    program::Arrangement::Set{
        name: std::borrow::Cow::from(r###"(Connected{.src=(_0: bit<32>), .dest=(_1: bit<32>)}: Connected) /*semijoin*/"###),
        fmfun: {fn __f(__v: DDValue) -> ::std::option::Option<DDValue>
        {
            match unsafe { <Connected>::from_ddvalue_unchecked(__v) } {
                Connected{src: ref _0, dest: ref _1} => Some((ddlog_std::tuple2((*_0).clone(), (*_1).clone())).into_ddvalue()),
                _ => None
            }
        }
        __f},
        distinct: false
    }
});
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
pub static __Rule_Connected_0 : ::once_cell::sync::Lazy<program::Rule> = ::once_cell::sync::Lazy::new(|| {
    /* Connected[(Connected{.src=src, .dest=dest}: Connected)] :- Edge[(Edge{.src=(src: bit<32>), .dest=(dest: bit<32>)}: Edge)]. */
    ::differential_datalog::program::Rule::CollectionRule {
        description: ::std::borrow::Cow::from("Connected(.src=src, .dest=dest) :- Edge(.src=src, .dest=dest). @ scc.dl:4:1"),
        rel: 1,
        xform: ::core::option::Option::Some(XFormCollection::FilterMap{
                                                description: std::borrow::Cow::from("head of Connected(.src=src, .dest=dest) :- Edge(.src=src, .dest=dest). @ scc.dl:4:1"),
                                                fmfun: {fn __f(__v: DDValue) -> ::std::option::Option<DDValue>
                                                {
                                                    let (ref src, ref dest) = match *unsafe { <Edge>::from_ddvalue_ref_unchecked(&__v) } {
                                                        Edge{src: ref src, dest: ref dest} => ((*src).clone(), (*dest).clone()),
                                                        _ => return ::core::option::Option::None
                                                    };
                                                    Some(((Connected{src: (*src).clone(), dest: (*dest).clone()})).into_ddvalue())
                                                }
                                                __f},
                                                next: Box::new(None)
                                            }),
    }
});
pub static __Rule_Connected_1 : ::once_cell::sync::Lazy<program::Rule> = ::once_cell::sync::Lazy::new(|| {
    /* Connected[(Connected{.src=src, .dest=dest}: Connected)] :- Edge[(Edge{.src=(src: bit<32>), .dest=(interum: bit<32>)}: Edge)], Connected[(Connected{.src=(interum: bit<32>), .dest=(dest: bit<32>)}: Connected)]. */
    ::differential_datalog::program::Rule::ArrangementRule {
        description: ::std::borrow::Cow::Borrowed("Connected(.src=src, .dest=dest) :- Edge(.src=src, .dest=interum), Connected(.src=interum, .dest=dest). @ scc.dl:5:1"),
        arr: (1, 0),
        xform: XFormArrangement::Join {
                   description: ::std::borrow::Cow::Borrowed("Edge(.src=src, .dest=interum), Connected(.src=interum, .dest=dest) @ scc.dl:5:1"),
                   ffun: None,
                   arrangement: (0,0),
                   jfun: {fn __f(_: &DDValue, __v1: &DDValue, __v2: &DDValue) -> ::std::option::Option<DDValue>
                   {
                       let (ref src, ref interum) = match *unsafe { <Edge>::from_ddvalue_ref_unchecked(__v1) } {
                           Edge{src: ref src, dest: ref interum} => ((*src).clone(), (*interum).clone()),
                           _ => return ::std::option::Option::None
                       };
                       let ref dest = match *unsafe { <Connected>::from_ddvalue_ref_unchecked(__v2) } {
                           Connected{src: _, dest: ref dest} => (*dest).clone(),
                           _ => return ::std::option::Option::None
                       };
                       ::std::option::Option::Some(((Connected{src: (*src).clone(), dest: (*dest).clone()})).into_ddvalue())
                   }
                   __f},
                   next: Box::new(::std::option::Option::None)
               },
    }
});
pub static __Rule_StronglyConnected_0 : ::once_cell::sync::Lazy<program::Rule> = ::once_cell::sync::Lazy::new(|| {
    /* StronglyConnected[(StronglyConnected{.node=node, .regime=regime}: StronglyConnected)] :- Connected[(Connected{.src=(node: bit<32>), .dest=(dest: bit<32>)}: Connected)], Connected[(Connected{.src=(dest: bit<32>), .dest=(node: bit<32>)}: Connected)], var __group = dest.group_by(node), ((var regime: bit<32>) = ((ddlog_std::group_min: function(ddlog_std::Group<bit<32>,bit<32>>):bit<32>)(__group))). */
    ::differential_datalog::program::Rule::ArrangementRule {
        description: ::std::borrow::Cow::Borrowed("StronglyConnected(.node=node, .regime=regime) :- Connected(.src=node, .dest=dest), Connected(.src=dest, .dest=node), var __group = dest.group_by(node), (var regime = (ddlog_std::group_min(__group))). @ scc.dl:8:1"),
        arr: (0, 1),
        xform: XFormArrangement::Semijoin {
                   description: ::std::borrow::Cow::Borrowed("Connected(.src=node, .dest=dest), Connected(.src=dest, .dest=node) @ scc.dl:8:1"),
                   ffun: None,
                   arrangement: (0,2),
                   jfun: {fn __f(_: &DDValue, __v1: &DDValue, ___v2: &()) -> ::std::option::Option<DDValue>
                   {
                       let (ref node, ref dest) = match *unsafe { <Connected>::from_ddvalue_ref_unchecked(__v1) } {
                           Connected{src: ref node, dest: ref dest} => ((*node).clone(), (*dest).clone()),
                           _ => return ::std::option::Option::None
                       };
                       ::std::option::Option::Some((ddlog_std::tuple2((*node).clone(), (*dest).clone())).into_ddvalue())
                   }
                   __f},
                   next: Box::new(::core::option::Option::Some(::differential_datalog::program::XFormCollection::Arrange {
                                                                   description: ::std::borrow::Cow::Borrowed("arrange Connected(.src=node, .dest=dest), Connected(.src=dest, .dest=node) @ scc.dl:8:1 by (node)"),
                                                                   afun: {
                                                                       fn __ddlog_generated_arrangement_function(__v: ::differential_datalog::ddval::DDValue) ->
                                                                           ::core::option::Option<(::differential_datalog::ddval::DDValue, ::differential_datalog::ddval::DDValue)>
                                                                       {
                                                                           let ddlog_std::tuple2(ref node, ref dest) = *unsafe { <ddlog_std::tuple2<u32, u32>>::from_ddvalue_ref_unchecked( &__v ) };
                                                                           ::core::option::Option::Some((((*node).clone()).into_ddvalue(), (ddlog_std::tuple2((*node).clone(), (*dest).clone())).into_ddvalue()))
                                                                       }
                                                                       __ddlog_generated_arrangement_function
                                                                   },
                                                                   next: ::std::boxed::Box::new(XFormArrangement::Aggregate{
                                                                                                    description: std::borrow::Cow::from("Connected(.src=node, .dest=dest), Connected(.src=dest, .dest=node), var __group = dest.group_by(node) @ scc.dl:8:1"),
                                                                                                    ffun: None,
                                                                                                    aggfun: {fn __f(__key: &DDValue, __group__: &[(&DDValue, Weight)]) -> ::std::option::Option<DDValue>
                                                                                                {
                                                                                                    let ref node = *unsafe { <u32>::from_ddvalue_ref_unchecked( __key ) };
                                                                                                    let ref __group = unsafe{ddlog_std::Group::new_by_ref((*node).clone(), __group__, {fn __f(__v: &DDValue) ->  u32
                                                                                                                                                                                      {
                                                                                                                                                                                          let ddlog_std::tuple2(ref node, ref dest) = *unsafe { <ddlog_std::tuple2<u32, u32>>::from_ddvalue_ref_unchecked( __v ) };
                                                                                                                                                                                          (*dest).clone()
                                                                                                                                                                                      }
                                                                                                                                                                                      ::std::sync::Arc::new(__f)})};
                                                                                                    let ref regime: u32 = match ddlog_std::group_min(__group) {
                                                                                                        regime => regime,
                                                                                                        _ => return None
                                                                                                    };
                                                                                                    Some((ddlog_std::tuple2((*regime).clone(), (*node).clone())).into_ddvalue())
                                                                                                }
                                                                                                __f},
                                                                                                    next: Box::new(::core::option::Option::Some(XFormCollection::FilterMap{
                                                                                                                                                    description: std::borrow::Cow::from("head of StronglyConnected(.node=node, .regime=regime) :- Connected(.src=node, .dest=dest), Connected(.src=dest, .dest=node), var __group = dest.group_by(node), (var regime = (ddlog_std::group_min(__group))). @ scc.dl:8:1"),
                                                                                                                                                    fmfun: {fn __f(__v: DDValue) -> ::std::option::Option<DDValue>
                                                                                                                                                    {
                                                                                                                                                        let ddlog_std::tuple2(ref regime, ref node) = *unsafe { <ddlog_std::tuple2<u32, u32>>::from_ddvalue_ref_unchecked( &__v ) };
                                                                                                                                                        Some(((StronglyConnected{node: (*node).clone(), regime: (*regime).clone()})).into_ddvalue())
                                                                                                                                                    }
                                                                                                                                                    __f},
                                                                                                                                                    next: Box::new(None)
                                                                                                                                                }))
                                                                                                }),
                                                               }))
               },
    }
});
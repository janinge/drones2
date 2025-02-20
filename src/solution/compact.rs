use crate::solution::Route;
use crate::types::CallId;

pub enum CompactIter<'a> {
    Compact {
        // We only need an immutable borrow here because the calls are all Some.
        compact_iter: std::iter::Map<
            std::slice::Iter<'a, Option<CallId>>,
            fn(&Option<CallId>) -> CallId,
        >,
    },
    NonCompact {
        route: &'a mut Route,
        read: usize,
        write: usize,
    },
}

impl<'a> CompactIter<'a> {
    pub fn new(route: &'a mut Route) -> Self {
        fn unwrap_call(opt: &Option<CallId>) -> CallId {
            opt.unwrap() // Safe because we check all elements are Some.
        }
        if route.calls.iter().all(|x| x.is_some()) {
            // In compact mode, we only need an immutable borrow.
            // It’s safe to temporarily borrow the calls field.
            let iter = route.calls.iter().map(unwrap_call as fn(&Option<CallId>) -> CallId);
            CompactIter::Compact { compact_iter: iter }
        } else {
            CompactIter::NonCompact {
                route,
                read: 0,
                write: 0,
            }
        }
    }
}

impl<'a> Iterator for CompactIter<'a> {
    type Item = CallId;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            CompactIter::Compact { compact_iter } => compact_iter.next(),
            CompactIter::NonCompact { route, read, write } => {
                while *read < route.calls.len() {
                    if let Some(call) = route.calls[*read] {
                        if *write != *read {
                            route.calls[*write] = Some(call);
                        }
                        *read += 1;
                        *write += 1;
                        return Some(call);
                    } else {
                        *read += 1;
                    }
                }
                route.calls.truncate(*write);
                None
            }
        }
    }
}
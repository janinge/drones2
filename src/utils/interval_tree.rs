use std::collections::BTreeMap;
use std::ops::RangeBounds;
use std::ops::RangeInclusive;

use crate::types::{CallId, Time};

#[derive(Default)]
pub struct IntervalTree {
    index_by_start: BTreeMap<Time, Vec<(Time, CallId)>>,
    index_by_end: BTreeMap<Time, Vec<(Time, CallId)>>,
}

impl IntervalTree {
    /// Constructs a new IntervalTree from an iterator over (CallId, Time window) pairs.
    ///
    /// For each interval, the start time is used as a key in `index_by_start` (with value (end, CallId))
    /// and the end time is used as a key in `index_by_end` (with value (start, CallId)).
    pub fn new<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (CallId, RangeInclusive<Time>)>,
    {
        let mut index_by_start = BTreeMap::new();
        let mut index_by_end = BTreeMap::new();

        for (call_id, window) in iter {
            let start = *window.start();
            let end = *window.end();
            index_by_start
                .entry(start)
                .or_insert_with(Vec::new)
                .push((end, call_id));
            index_by_end
                .entry(end)
                .or_insert_with(Vec::new)
                .push((start, call_id));
        }

        Self {
            index_by_start,
            index_by_end,
        }
    }

    /// Private helper that iterates over the given map for keys in the given range,
    /// optionally applying a predicate to each entry.
    /// If no predicate is provided, all call IDs in the range are collected.
    fn collect_from_map<F, R>(
        &self,
        map: &BTreeMap<Time, Vec<(Time, CallId)>>,
        range: R,
        predicate: Option<F>,
    ) -> Vec<CallId>
    where
        F: Fn(&(Time, CallId)) -> bool,
        R: RangeBounds<Time>,
    {
        let mut result = Vec::new();
        for (_key, entries) in map.range(range) {
            for entry in entries {
                if predicate.as_ref().map_or(true, |pred| pred(entry)) {
                    result.push(entry.1);
                }
            }
        }
        result
    }

    /// Active query using the start-time index.
    /// Returns all call IDs whose interval has started (key ≤ current_time)
    /// and not yet expired (current_time ≤ stored end).
    pub fn query_by_start(&self, current_time: Time) -> Vec<CallId> {
        self.collect_from_map(
            &self.index_by_start,
            ..=current_time,
            Some(move |entry: &(Time, CallId)| -> bool { current_time <= entry.0 }),
        )
    }

    /// Active query using the end-time index.
    /// Returns all call IDs whose interval has not yet expired (key ≥ current_time)
    /// and has started (current_time ≥ stored start).
    pub fn query_by_end(&self, current_time: Time) -> Vec<CallId> {
        self.collect_from_map(
            &self.index_by_end,
            current_time..,
            Some(move |entry: &(Time, CallId)| -> bool { current_time >= entry.0 }),
        )
    }

    /// General active query that accepts a flag to choose which underlying index to use.
    /// (Under active conditions both queries should return the same result.)
    pub fn query(&self, current_time: Time, use_start_index: bool) -> Vec<CallId> {
        if use_start_index {
            self.query_by_start(current_time)
        } else {
            self.query_by_end(current_time)
        }
    }

    /// Convenience method that queries using the end-time index.
    pub fn query_default(&self, current_time: Time) -> Vec<CallId> {
        self.query(current_time, false)
    }

    /// Returns all call IDs with a start time ≤ the given query time.
    pub fn query_start_before(&self, query_time: Time) -> Vec<CallId> {
        self.collect_from_map(
            &self.index_by_start,
            ..=query_time,
            None::<fn(&(Time, CallId)) -> bool>,
        )
    }

    /// Returns all call IDs with a start time ≥ the given query time.
    pub fn query_start_after(&self, query_time: Time) -> Vec<CallId> {
        self.collect_from_map(
            &self.index_by_start,
            query_time..,
            None::<fn(&(Time, CallId)) -> bool>,
        )
    }

    /// Returns all call IDs with an end time ≤ the given query time.
    pub fn query_end_before(&self, query_time: Time) -> Vec<CallId> {
        self.collect_from_map(
            &self.index_by_end,
            ..=query_time,
            None::<fn(&(Time, CallId)) -> bool>,
        )
    }

    /// Returns all call IDs with an end time ≥ the given query time.
    pub fn query_end_after(&self, query_time: Time) -> Vec<CallId> {
        self.collect_from_map(
            &self.index_by_end,
            query_time..,
            None::<fn(&(Time, CallId)) -> bool>,
        )
    }

    /// Returns an iterator over all (time, call) with `start_time ≥ from` in ascending order.
    pub fn start_events_from(&self, from: Time)
                             -> impl Iterator<Item = (Time, CallId)> + '_
    {
        self.index_by_start
            .range(from..)
            .flat_map(|(&t_start, vec)| {
                vec.iter().map(move |&(_end, call)| (t_start, call))
            })
    }

    /// Same for end‐times ≥ from
    pub fn end_events_from(&self, from: Time)
                           -> impl Iterator<Item = (Time, CallId)> + '_
    {
        self.index_by_end
            .range(from..)
            .flat_map(|(&t_end, vec)| {
                vec.iter().map(move |&(_start, call)| (t_end, call))
            })
    }
}

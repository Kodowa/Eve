use std::collections::BTreeSet;

use value::Value;
use relation::{Relation, IndexSelect, ViewSelect};
use primitive::{Primitive, resolve_as_scalar};
use std::cmp::{min, max};

#[derive(Clone, Debug)]
pub struct Table {
    pub insert: Option<IndexSelect>,
    pub remove: Option<IndexSelect>,
}

#[derive(Clone, Debug)]
pub struct Union {
    pub selects: Vec<IndexSelect>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstraintOp {
    EQ,
    NEQ,
    LT,
    GT,
    LTE,
    GTE,
}

#[derive(Clone, Debug)]
pub struct Constraint {
    pub left: usize,
    pub op: ConstraintOp,
    pub right: usize,
}

impl Constraint {
    fn is_satisfied_by(&self, state: &[&Value]) -> bool {
        match self.op {
            ConstraintOp::EQ => state[self.left] == state[self.right],
            ConstraintOp::NEQ => state[self.left] != state[self.right],
            ConstraintOp::LT => state[self.left] < state[self.right],
            ConstraintOp::GT => state[self.left] > state[self.right],
            ConstraintOp::LTE => state[self.left] <= state[self.right],
            ConstraintOp::GTE => state[self.left] >= state[self.right],
        }
    }
}

#[derive(Clone, Debug)]
pub enum JoinSource {
    Relation{
        input: usize
    },
    Primitive{
        primitive: Primitive,
        arguments: Vec<usize>,
    },
}

#[derive(Clone, Debug)]
pub struct Join {
    pub constants: Vec<Value>,
    pub sources: Vec<JoinSource>,
    pub constraints: Vec<Vec<Constraint>>,
    pub select: ViewSelect,
}

#[derive(Clone, Debug)]
pub struct Reducer {
    pub primitive: Primitive,
    pub arguments: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct Aggregate {
    pub constants: Vec<Value>,
    pub outer: IndexSelect,
    pub inner: IndexSelect,
    pub limit_from: Option<usize>,
    pub limit_to: Option<usize>,
    pub reducers: Vec<Reducer>,
    pub selects_inner: bool,
    pub select: ViewSelect,
}

#[derive(Clone, Debug)]
pub enum View {
    Table(Table),
    Union(Union),
    Join(Join),
    Aggregate(Aggregate),
}

fn push_all<'a>(state: &mut Vec<&'a Value>, input: &'a Vec<Value>) {
    for value in input.iter() {
        state.push(value);
    }
}

fn pop_all<'a>(state: &mut Vec<&'a Value>, input: &'a Vec<Value>) {
    for _ in input.iter() {
        state.pop();
    }
}

fn join_step<'a>(join: &'a Join, ix: usize, inputs: &[&'a Relation], state: &mut Vec<&'a Value>, index: &mut BTreeSet<Vec<Value>>) {
    if ix == join.sources.len() {
        index.insert(join.select.select(&state[..]));
    } else {
        match join.sources[ix] {
            JoinSource::Relation{input} => {
                for values in inputs[input].index.iter() {
                    push_all(state, values);
                    if join.constraints[ix].iter().all(|constraint| constraint.is_satisfied_by(&state[..])) {
                        join_step(join, ix+1, inputs, state, index)
                    }
                    pop_all(state, values);
                }
            }
            JoinSource::Primitive{ref primitive, ref arguments, ..} => {
                for values in primitive.eval_from_join(&arguments[..], &state[..]).into_iter() {
                    // promise the borrow checker that we will pop values before we exit this scope
                    let values = unsafe { ::std::mem::transmute::<&Vec<Value>, &'a Vec<Value>>(&values) };
                    push_all(state, values);
                    if join.constraints[ix].iter().all(|constraint| constraint.is_satisfied_by(&state[..])) {
                        join_step(join, ix+1, inputs, state, index)
                    }
                    pop_all(state, values);
                }
            }
        }
    }
}

fn aggregate_step<'a>(aggregate: &Aggregate, input_sets: &'a [&[Vec<Value>]], state: &mut Vec<&'a Value>, index: &mut BTreeSet<Vec<Value>>) {
    if input_sets.len() == 0 {
        index.insert(aggregate.select.select(&state[..]));
    } else {
        for values in input_sets[0].iter() {
            push_all(state, values);
            aggregate_step(aggregate, &input_sets[1..], state, index);
            pop_all(state, values);
        }
    }
}

impl View {
    pub fn run(&self, old_output: &Relation, inputs: &[&Relation]) -> Option<Relation> {
        match *self {
            View::Table(_) => None,
            View::Union(ref union) => {
                assert_eq!(union.selects.len(), inputs.len());
                let mut output = Relation::with_fields(old_output.fields.clone(), old_output.names.clone());
                for select in union.selects.iter() {
                    for values in select.select(&inputs[..]) {
                        output.index.insert(values);
                    }
                }
                Some(output)
            }
            View::Join(ref join) => {
                let mut output = Relation::with_fields(old_output.fields.clone(), old_output.names.clone());
                let mut tuples = Vec::with_capacity(join.sources.len());
                join_step(join, 0, inputs, &mut tuples, &mut output.index);
                Some(output)
            }
            View::Aggregate(ref aggregate) => {
                let mut output = Relation::with_fields(old_output.fields.clone(), old_output.names.clone());
                let mut outer = aggregate.outer.select(&inputs[..]);
                let mut inner = aggregate.inner.select(&inputs[..]);
                outer.sort();
                outer.dedup();
                inner.sort();
                let constants = &aggregate.constants[..];
                let mut group_start = 0;
                for outer_values in outer.into_iter() {
                    let mut group_end = group_start;
                    while (group_end < inner.len())
                    && (inner[group_end][0..outer_values.len()] == outer_values[..]) {
                        group_end += 1;
                    }
                    let (group, output_values) = {
                        let limit_from = match aggregate.limit_from {
                            None => group_start,
                            Some(ix) => group_start
                                + resolve_as_scalar(ix, constants, &outer_values[..]).as_usize(),
                        };
                        let limit_to = match aggregate.limit_to {
                            None => group_end,
                            Some(ix) => group_start
                                + resolve_as_scalar(ix, constants, &outer_values[..]).as_usize(),
                        };
                        let limit_from = min(max(limit_from, group_start), group_end);
                        let limit_to = min(max(limit_to, limit_from), group_end);
                        let group = &inner[limit_from..limit_to];
                        let output_values = aggregate.reducers.iter().map(|reducer| {
                            reducer.primitive.eval_from_aggregate(&reducer.arguments[..], constants, &outer_values[..], group)
                        }).collect::<Vec<_>>();
                        (group, output_values)
                    };
                    let mut output_sets = vec![];
                    if aggregate.selects_inner {
                        output_sets.push(group);
                    }
                    for output in output_values.iter() {
                        output_sets.push(output);
                    }
                    let mut state = outer_values.iter().collect();
                    aggregate_step(aggregate, &output_sets[..], &mut state, &mut output.index);
                    group_start = group_end;
                }
                Some(output)
            }
        }
    }
}
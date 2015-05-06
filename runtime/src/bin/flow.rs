extern crate eve;

use eve::index::Index;
use eve::value::Value::*;
use eve::query::*;
use eve::flow::*;
use eve::test::*;

use std::cell::RefCell;

#[allow(dead_code)]
fn main() {
    let edges = vec![("a","b"), ("b", "c"), ("c", "d"), ("d", "b")];
    let edge_union = Union{mappings:vec![]};
    let path_union = Union{
        mappings: vec![
        (2, vec![Ref::Value{clause: 0, column: 0}, Ref::Value{clause: 1, column: 1}]),
        (1, vec![Ref::Value{clause: 0, column: 0}, Ref::Value{clause: 0, column: 1}])
        ],
    };
    let first_step_query = Query{clauses: vec![
        Clause::Tuple(Source{relation: 0, constraints: vec![]}),
    ]};
    let from_eq_to = Constraint{
        my_column: 0,
        op: ConstraintOp::EQ,
        other_ref: Ref::Value{
            clause: 0,
            column: 1,
        }
    };
    let next_step_query = Query{clauses: vec![
        Clause::Tuple(Source{relation: 0, constraints: vec![]}),
        Clause::Tuple(Source{relation: 1, constraints: vec![from_eq_to]}),
    ]};
    let mut flow = Flow{
        nodes: vec![
            Node{
                id: "edge".to_string(),
                view: View::Union(edge_union),
                upstream: vec![],
                downstream: vec![2,3],
            },
            Node{
                id: "path".to_string(),
                view: View::Union(path_union),
                upstream: vec![2,3],
                downstream: vec![2],
            },
            Node{
                id: "next_step".to_string(),
                view: View::Query(next_step_query),
                upstream: vec![0,1],
                downstream: vec![1],
            },
            Node{
                id: "first_step".to_string(),
                view: View::Query(first_step_query),
                upstream: vec![0],
                downstream: vec![1],
            },
        ],
        inputs: vec![
            RefCell::new(edges.to_relation()),
            RefCell::new(Index::new()),
            RefCell::new(Index::new()),
            RefCell::new(Index::new()),
            ],
        outputs: vec![
            RefCell::new(Index::new()),
            RefCell::new(Index::new()),
            RefCell::new(Index::new()),
            RefCell::new(Index::new()),
            ],
        dirty: vec![0,1,2,3].into_iter().collect(),
        changes: Vec::new(),
    };
    flow.run();
    println!("{:?}", flow.changes);
    println!("{:?}", flow.get_output("path"));
}
// fixtures/tables/mod.rs
//
// This module contains definitions for test tables.
// Each test table is a struct that implements TestTable trait.
//
// What we're doing:
// We define a "template" for a table once, then reuse it in many tests.
// This follows the DRY (Don't Repeat Yourself) principle.

pub mod products;

pub use products::ProductsTable;



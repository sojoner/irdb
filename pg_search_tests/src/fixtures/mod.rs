// fixtures/mod.rs - Test fixtures module
//
// What is a fixture?
// A fixture is reusable test setup code. Instead of writing the same database
// setup in every test, we write it once and reuse it.
//
// In Rust, we use the `rstest` crate for fixtures. Think of it like:
// - Fixtures are like "templates" for your tests
// - They automatically run before each test
// - They can return values that the test needs
//
// Example: Instead of this in every test...
//   let conn = connect_to_db().await;
//   create_table(&conn).await;
//   insert_test_data(&conn).await;
//
// We just use: #[rstest] and the fixture does all that for us!

pub mod tables;

/// A simple trait that all test tables must implement
pub trait TestTable {
    /// The SQL commands to create and populate this table
    /// Returns a slice of SQL strings that should be executed in order
    fn setup_sql() -> &'static [&'static str];
}
//! Use this file for iterating on the derive code. You can view the
//! expanded code for any given configuration by updating this file and
//! running:
//!
//! ```sh
//! cargo expand --test=prototype
//! ```


#[test_tag::tag(tag1, tag2)]
#[test_tag::tag(tag3)]
#[test]
fn it_works() {
  assert_eq!(2 + 2, 4);
}

//#[test_tag::tag(miri)]
//#[test]
//fn it_works_as_well() {
//  assert_eq!(2 + 2, 4);
//}
//
//#[test_tag::tag(mirinot)]
//#[test]
//fn it_works_not() {
//  assert_eq!(2 + 2, 3);
//}
//
//#[test_tag::tag(notmiri)]
//#[test]
//fn it_works_not2() {
//  assert_eq!(2 + 2, 3);
//}

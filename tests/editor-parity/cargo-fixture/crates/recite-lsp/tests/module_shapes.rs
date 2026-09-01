// #[test] fn fake_test_in_a_line_comment() {}
const RAW_TEST_TEXT: &str = r##"#[test] fn fake_test_in_a_raw_string() {}"##;

mod inline {
    #[test]
    fn inline_test() {}

    mod nested {
        #[test]
        fn nested_test() {}
    }
}

mod support;
mod support_directory;
#[path = "support/path_support.rs"]
mod path_support;
#[cfg(any())]
mod cfg_disabled;

#[test]
fn root_test() {}

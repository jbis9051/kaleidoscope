use common::media_query::JoinableTable;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use common::media_query::macros::DSLType;
use common::media_query::macros::{format_value, parse_filter};
use common::{dsl_types, query_dsl};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;


#[test]
pub fn test_parse_filter() {
    // filter: num:>=10 str:%foo date:=2020-01-01

    let tests = [
        (
            "num:>=10 str:%foo date:=2020-01-01",
            vec!["num:>=10", "str:%foo", "date:=2020-01-01"],
        ),
        (
            "str:='foo' date:=2020-01-01",
            vec!["str:=foo", "date:=2020-01-01"],
        ),
        (
            "  str:%\"foo\"  date:=2020-01-01",
            vec!["str:%foo", "date:=2020-01-01"],
        ),
        (r#"str:%'\' foo \''"#, vec![r#"str:%' foo '"#]),
        (r#"str:%"foo foo""#, vec![r#"str:%foo foo"#]),
        ("", vec![]),
        (
            r#"str:%"foo's""#,
            vec![r#"str:%foo's"#],
        )
    ];

    for (input, expected) in tests.iter() {
        let parsed = parse_filter(input).expect("failed to parse filter");
        assert_eq!(parsed, *expected, "input: {}", input);
    }

    let errors = ["str:'foo", "str:foo'", r#"str:j\"#];

    for input in errors.iter() {
        let parsed = parse_filter(input);
        assert!(parsed.is_err(), "input: {}", input);
    }
}

#[test]
pub fn test_dsl() {
    dsl_types! {
        number(DSLNum, i32) {
            GreaterEqual = ">=",
            LessEqual = "<=",
            Greater = ">",
            Less = "<",
            Equal = "=",
            |x| {
                Ok(x.parse().map_err(|_| format!("invalid number format: {}", x))?)
            }
        };
        string(DSLString, String) {
            Equal = "=",
            Like = "%",
            NotLike = "!%",
            |x| {
                Ok(x.to_string())
            }
        };
        date(DSLDate, NaiveDate) {
            Equal = "=",
            Before = "<",
            After = ">",
            |x| {
                // 2020-01-01
                Ok(NaiveDate::parse_from_str(x, "%Y-%m-%d").map_err(|_| "invalid date format".to_string())?)
            }
        };
    }
    query_dsl!(
        TestDSLQuery(TestDSLQueryEnum) {
            num(number, Num, []),
            str(string, Str, []),
            date(date, Date, []),
        }
    );

    let tests = [
        (
            "num:>=10 str:%foo date:=2020-01-01",
            vec![
                TestDSLQueryEnum::Num(DSLNum::GreaterEqual, 10),
                TestDSLQueryEnum::Str(DSLString::Like, "foo".to_string()),
                TestDSLQueryEnum::Date(
                    DSLDate::Equal,
                    NaiveDate::parse_from_str("2020-01-01", "%Y-%m-%d").unwrap(),
                ),
            ],
        ),
        (
            "num:<10 str:=foo date:>1955-01-01",
            vec![
                TestDSLQueryEnum::Num(DSLNum::Less, 10),
                TestDSLQueryEnum::Str(DSLString::Equal, "foo".to_string()),
                TestDSLQueryEnum::Date(
                    DSLDate::After,
                    NaiveDate::parse_from_str("1955-01-01", "%Y-%m-%d").unwrap(),
                ),
            ],
        ),
        (
            "num:<=10 str:%foo",
            vec![
                TestDSLQueryEnum::Num(DSLNum::LessEqual, 10),
                TestDSLQueryEnum::Str(DSLString::Like, "foo".to_string()),
            ],
        ),
        (
            "str:='foo' date:=2020-01-01",
            vec![
                TestDSLQueryEnum::Str(DSLString::Equal, "foo".to_string()),
                TestDSLQueryEnum::Date(
                    DSLDate::Equal,
                    NaiveDate::parse_from_str("2020-01-01", "%Y-%m-%d").unwrap(),
                ),
            ],
        ),
        (
            "  str:%\"foo\"  date:=2020-01-01",
            vec![
                TestDSLQueryEnum::Str(DSLString::Like, "foo".to_string()),
                TestDSLQueryEnum::Date(
                    DSLDate::Equal,
                    NaiveDate::parse_from_str("2020-01-01", "%Y-%m-%d").unwrap(),
                ),
            ],
        ),
        (
            r#"str:%'\' foo \''"#,
            vec![TestDSLQueryEnum::Str(
                DSLString::Like,
                r#"' foo '"#.to_string(),
            )],
        ),
        (
            "str:%foo str:!%bar num:=5 num:=2",
            vec![
                TestDSLQueryEnum::Str(DSLString::Like, "foo".to_string()),
                TestDSLQueryEnum::Str(DSLString::NotLike, "bar".to_string()),
                TestDSLQueryEnum::Num(DSLNum::Equal, 5),
                TestDSLQueryEnum::Num(DSLNum::Equal, 2),
            ],
        ),
        (
            r#"str:%"foo\'s""#,
            vec![TestDSLQueryEnum::Str(
                DSLString::Like,
                r#"foo's"#.to_string(),
            )],
        ),
        (
            r#"str:%"fo\o\'s""#,
            vec![TestDSLQueryEnum::Str(
                DSLString::Like,
                r#"foo's"#.to_string(),
            )],
        ),
        (
            r#"str:%"foo's""#,
            vec![TestDSLQueryEnum::Str(
                DSLString::Like,
                r#"foo's"#.to_string(),
            )],
        ),
        (
            r#"str:%"%foo%""#,
            vec![TestDSLQueryEnum::Str(
                DSLString::Like,
                r#"%foo%"#.to_string(),
            )],
        ),
        
    ];

    let errors = [
        "str:='foo",
        "str:=foo'",
        r#"str:=j\"#,
        "num:=foo",
        "date:=foo",
        "num:3",
        "str:%foo str:!%bar num=5 num=2",
    ];

    println!("{}", TestDSLQueryEnum::describe());

    for (input, expected) in tests.iter() {
        let parsed: TestDSLQuery = input.parse().expect("failed to parse query");
        assert_eq!(&parsed.filters, expected, "input: {}", input);
        assert!(!parsed.to_string().is_empty())
    }

    for input in errors.iter() {
        let parsed: Result<TestDSLQuery, _> = input.parse();
        assert!(parsed.is_err(), "input: {}", input);
    }

    // test json serialization

    #[derive(Debug, Serialize, Deserialize)]
    struct TestStruct {
        query: TestDSLQuery,
    }

    let input = "num:>=10 str:%foo date:=2020-01-01";
    let good: TestDSLQuery = input.parse().expect("failed to parse query");
    let json = format!(r#"{{"query":"{}"}}"#, input);
    let parsed: TestStruct = serde_json::from_str(&json).expect("failed to parse json");
    assert_eq!(parsed.query.filters, good.filters);
}

#[test]
pub fn media_query_validation() {
    let tests = [(
         "created_at:>2020-01-01 is_screenshot:=false has_gps:=true filter_path:%'foo%' filter_path:!%'%.jpg' order_by:=created_at asc:=true limit:=10 page:=1",
          true
        ),
        (
            "created_at:>2020-01-01 is_screenshot:=false has_gps:=true filter_path:%'foo%' filter_path:!%'%.jpg' order_by:=created_at asc:=true page:=1",
            false
            ),
        (
            "order_by:=created_at created_at:>2020-01-01 is_screenshot:=false has_gps:=true filter_path:%'foo%' filter_path:!%'%.jpg' asc:=true limit:=10 page:=1",
            false
        ),
        (
            "created_at:>2020-01-01 is_screenshot:=false has_gps:=true filter_path:%'foo%' filter_path:!%'%.jpg' order_by:=created_at asc:=true limit:=10 page:=1 page:=2",
            false
        ),
        (
            "created_at:>2020-01-01 is_screenshot:=false has_gps:=true filter_path:%'foo%' filter_path:!%'%.jpg' order_by:=created_at asc:=true page:=1 limit:=10",
            false
        ),
        (
            "created_at:>2020-01-01 is_screenshot:=false has_gps:=true filter_path:%'foo%' filter_path:!%'%.jpg' limit:=10 page:=1 order_by:=created_at asc:=true",
            false
        ),
    ];

    for (input, expected) in tests.iter() {
        let parsed = input.parse::<common::media_query::media_query::MediaQuery>();
        assert!(parsed.is_ok(), "input: {:?} - {}", parsed, input);
        let validate = parsed.unwrap().validate();
        assert_eq!(
            validate.is_ok(),
            *expected,
            "input: {:?} - {}",
            validate,
            input
        );
    }
}

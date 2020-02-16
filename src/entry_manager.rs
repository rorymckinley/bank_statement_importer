use crate::Sphere;
use crate::ui_choices::{UiChoices, PatternOverride};
use crate::raw_entry::{Direction};

#[cfg(test)]
mod tests {
    use csv::StringRecord;
    use crate::raw_entry::RawEntry;
    use super::*;

    #[test]
    fn test_create_classification_from_choices_existing_pattern_override() {
        let entry = RawEntry::new(
            StringRecord::from(vec!["  20191101  ", "  foo one bar  ", "  191.60  ", "  200.00  "])
            );
        let pattern = Pattern::Inbound { snippet: "foo".into(), category: "baz".into(), assign_as_income: true };
        let expected_output = Classification::ExistingPattern(&pattern, Some(PatternOverride {is_personal: true}));
        let choices = UiChoices {
            existing_pattern: Some(&pattern),
            pattern_override: Some(PatternOverride {is_personal: true}),
            category: None,
            transfer: None,
            sphere: None,
            create_pattern: None,
            snippet: None,
            require_confirmation: None
        };
        assert_eq!(
            expected_output,
            create_classification_from_choices(entry.direction, choices)
            )
    }

    #[test]
    fn test_create_classification_from_choices_existing_pattern_no_override() {
        let entry = RawEntry::new(
            StringRecord::from(vec!["  20191101  ", "  foo one bar  ", "  191.60  ", "  200.00  "])
            );
        let pattern = Pattern::Inbound { snippet: "foo".into(), category: "baz".into(), assign_as_income: true };
        let expected_output = Classification::ExistingPattern(&pattern, None);
        let choices = UiChoices {
            existing_pattern: Some(&pattern),
            pattern_override: None,
            category: None,
            transfer: None,
            sphere: None,
            create_pattern: None,
            snippet: None,
            require_confirmation: None
        };
        assert_eq!(
            expected_output,
            create_classification_from_choices(entry.direction, choices)
            )
    }

    #[test]
    fn test_create_classification_from_choices_new_inbound_pattern_assign_as_income() {
        let entry = RawEntry::new(
            StringRecord::from(vec!["  20191101  ", "  foo one bar  ", "  191.60  ", "  200.00  "])
            );
        let expected_output = Classification::NewPatternInbound {
            snippet: "snip".into(), category: "foo".into(), assign_as_income: true
        };
        let choices = UiChoices {
            existing_pattern: None,
            pattern_override: None,
            category: Some("foo".into()),
            transfer: Some(false),
            sphere: Some(Sphere::Personal),
            create_pattern: Some(true),
            snippet: Some("snip".into()),
            require_confirmation: Some(false)
        };
        assert_eq!(
            expected_output,
            create_classification_from_choices(entry.direction, choices)
            )
    }

    #[test]
    fn test_create_classification_from_choices_new_inbound_pattern_assign_as_transfer() {
        let entry = RawEntry::new(
            StringRecord::from(vec!["  20191101  ", "  foo one bar  ", "  191.60  ", "  200.00  "])
            );
        let expected_output = Classification::NewPatternInbound {
            snippet: "snip".into(), category: "foo".into(), assign_as_income: false
        };
        let choices = UiChoices {
            existing_pattern: None,
            pattern_override: None,
            category: Some("foo".into()),
            transfer: Some(true),
            sphere: Some(Sphere::Personal),
            create_pattern: Some(true),
            snippet: Some("snip".into()),
            require_confirmation: Some(false)
        };
        assert_eq!(
            expected_output,
            create_classification_from_choices(entry.direction, choices)
            )
    }

    #[test]
    fn test_create_classification_from_choices_new_outbound_pattern_personal_expense_requiring_confirmation() {
        let entry = RawEntry::new(
            StringRecord::from(vec!["  20191101  ", "  foo one bar  ", "  -191.60  ", "  200.00  "])
            );
        let expected_output = Classification::NewPatternOutbound {
            snippet: "snip".into(),
            category: "foo".into(),
            assign_as_expense: true,
            assign_as_personal: true,
            require_confirmation: true
        };
        let choices = UiChoices {
            existing_pattern: None,
            pattern_override: None,
            category: Some("foo".into()),
            transfer: Some(false),
            sphere: Some(Sphere::Personal),
            create_pattern: Some(true),
            snippet: Some("snip".into()),
            require_confirmation: Some(true)
        };
        assert_eq!(
            expected_output,
            create_classification_from_choices(entry.direction, choices)
            )
    }

    #[test]
    fn test_create_classification_from_choices_new_outbound_pattern_personal_expense_no_confirmation() {
        let entry = RawEntry::new(
            StringRecord::from(vec!["  20191101  ", "  foo one bar  ", "  -191.60  ", "  200.00  "])
            );
        let expected_output = Classification::NewPatternOutbound {
            snippet: "snip".into(),
            category: "foo".into(),
            assign_as_expense: true,
            assign_as_personal: true,
            require_confirmation: false
        };
        let choices = UiChoices {
            existing_pattern: None,
            pattern_override: None,
            category: Some("foo".into()),
            transfer: Some(false),
            sphere: Some(Sphere::Personal),
            create_pattern: Some(true),
            snippet: Some("snip".into()),
            require_confirmation: Some(false)
        };
        assert_eq!(
            expected_output,
            create_classification_from_choices(entry.direction, choices)
            )
    }

    #[test]
    fn test_create_classification_from_choices_new_outbound_pattern_personal_transfer() {
        let entry = RawEntry::new(
            StringRecord::from(vec!["  20191101  ", "  foo one bar  ", "  -191.60  ", "  200.00  "])
            );
        let expected_output = Classification::NewPatternOutbound {
            snippet: "snip".into(),
            category: "foo".into(),
            assign_as_expense: false,
            assign_as_personal: true,
            require_confirmation: false
        };
        let choices = UiChoices {
            existing_pattern: None,
            pattern_override: None,
            category: Some("foo".into()),
            transfer: Some(true),
            sphere: Some(Sphere::Personal),
            create_pattern: Some(true),
            snippet: Some("snip".into()),
            require_confirmation: Some(false)
        };
        assert_eq!(
            expected_output,
            create_classification_from_choices(entry.direction, choices)
            )
    }

    #[test]
    fn test_create_classification_from_choices_new_outbound_pattern_work_expense_requires_confirmation() {
        let entry = RawEntry::new(
            StringRecord::from(vec!["  20191101  ", "  foo one bar  ", "  -191.60  ", "  200.00  "])
            );
        let expected_output = Classification::NewPatternOutbound {
            snippet: "snip".into(),
            category: "foo".into(),
            assign_as_expense: true,
            assign_as_personal: false,
            require_confirmation: true
        };
        let choices = UiChoices {
            existing_pattern: None,
            pattern_override: None,
            category: Some("foo".into()),
            transfer: Some(false),
            sphere: Some(Sphere::Work),
            create_pattern: Some(true),
            snippet: Some("snip".into()),
            require_confirmation: Some(true)
        };
        assert_eq!(
            expected_output,
            create_classification_from_choices(entry.direction, choices)
            )
    }

    #[test]
    fn test_create_classification_from_choices_new_outbound_pattern_work_expense_no_confirmation() {
        let entry = RawEntry::new(
            StringRecord::from(vec!["  20191101  ", "  foo one bar  ", "  -191.60  ", "  200.00  "])
            );
        let expected_output = Classification::NewPatternOutbound {
            snippet: "snip".into(),
            category: "foo".into(),
            assign_as_expense: true,
            assign_as_personal: false,
            require_confirmation: false
        };
        let choices = UiChoices {
            existing_pattern: None,
            pattern_override: None,
            category: Some("foo".into()),
            transfer: Some(false),
            sphere: Some(Sphere::Work),
            create_pattern: Some(true),
            snippet: Some("snip".into()),
            require_confirmation: Some(false)
        };
        assert_eq!(
            expected_output,
            create_classification_from_choices(entry.direction, choices)
            )
    }

    #[test]
    fn test_create_classification_from_choices_no_pattern_inbound_income() {
        let entry = RawEntry::new(
            StringRecord::from(vec!["  20191101  ", "  foo one bar  ", "  191.60  ", "  200.00  "])
            );
        let expected_output = Classification::NoPatternInbound {
            category: "foo".into(),
            assign_as_income: true,
        };
        let choices = UiChoices {
            existing_pattern: None,
            pattern_override: None,
            category: Some("foo".into()),
            transfer: Some(false),
            sphere: Some(Sphere::Personal),
            create_pattern: Some(false),
            snippet: None,
            require_confirmation: None
        };
        assert_eq!(
            expected_output,
            create_classification_from_choices(entry.direction, choices)
            )
    }

    #[test]
    fn test_create_classification_from_choices_no_pattern_inbound_transfer() {
        let entry = RawEntry::new(
            StringRecord::from(vec!["  20191101  ", "  foo one bar  ", "  191.60  ", "  200.00  "])
            );
        let expected_output = Classification::NoPatternInbound {
            category: "foo".into(),
            assign_as_income: false,
        };
        let choices = UiChoices {
            existing_pattern: None,
            pattern_override: None,
            category: Some("foo".into()),
            transfer: Some(true),
            sphere: Some(Sphere::Personal),
            create_pattern: Some(false),
            snippet: None,
            require_confirmation: None
        };
        assert_eq!(
            expected_output,
            create_classification_from_choices(entry.direction, choices)
            )
    }

    #[test]
    fn test_create_classification_from_choices_no_pattern_outbound_personal_expense() {
        let entry = RawEntry::new(
            StringRecord::from(vec!["  20191101  ", "  foo one bar  ", "  -191.60  ", "  200.00  "])
            );
        let expected_output = Classification::NoPatternOutbound {
            category: "foo".into(),
            assign_as_expense: true,
            assign_as_personal: true,
        };
        let choices = UiChoices {
            existing_pattern: None,
            pattern_override: None,
            category: Some("foo".into()),
            transfer: Some(false),
            sphere: Some(Sphere::Personal),
            create_pattern: Some(false),
            snippet: None,
            require_confirmation: None
        };
        assert_eq!(
            expected_output,
            create_classification_from_choices(entry.direction, choices)
            )
    }

    #[test]
    fn test_create_classification_from_choices_no_pattern_outbound_work_expense() {
        let entry = RawEntry::new(
            StringRecord::from(vec!["  20191101  ", "  foo one bar  ", "  -191.60  ", "  200.00  "])
            );
        let expected_output = Classification::NoPatternOutbound {
            category: "foo".into(),
            assign_as_expense: true,
            assign_as_personal: false,
        };
        let choices = UiChoices {
            existing_pattern: None,
            pattern_override: None,
            category: Some("foo".into()),
            transfer: Some(false),
            sphere: Some(Sphere::Work),
            create_pattern: Some(false),
            snippet: None,
            require_confirmation: None
        };
        assert_eq!(
            expected_output,
            create_classification_from_choices(entry.direction, choices)
            )
    }

    #[test]
    fn test_create_classification_from_choices_no_pattern_outbound_transfer() {
        let entry = RawEntry::new(
            StringRecord::from(vec!["  20191101  ", "  foo one bar  ", "  -191.60  ", "  200.00  "])
            );
        let expected_output = Classification::NoPatternOutbound {
            category: "foo".into(),
            assign_as_expense: false,
            assign_as_personal: true,
        };
        let choices = UiChoices {
            existing_pattern: None,
            pattern_override: None,
            category: Some("foo".into()),
            transfer: Some(true),
            sphere: Some(Sphere::Personal),
            create_pattern: Some(false),
            snippet: None,
            require_confirmation: None
        };
        assert_eq!(
            expected_output,
            create_classification_from_choices(entry.direction, choices)
            )
    }
}

pub fn create_classification_from_choices(direction: Direction, choices: UiChoices) -> Classification {
    match choices.existing_pattern {
        Some(pattern) => {
            Classification::ExistingPattern(pattern, choices.pattern_override)
        },
        _ => {
            match direction {
                Direction::Inbound => {
                    match choices.create_pattern.unwrap() {
                        true => {
                            Classification::NewPatternInbound {
                                snippet: choices.snippet.unwrap(),
                                category: choices.category.unwrap(),
                                assign_as_income: !choices.transfer.unwrap()
                            }
                        },
                        false => {
                            Classification::NoPatternInbound {
                                category: choices.category.unwrap(),
                                assign_as_income: !choices.transfer.unwrap()
                            }
                        }
                    }
                },
                Direction::Outbound => {
                    match choices.create_pattern.unwrap() {
                        true => {
                            Classification::NewPatternOutbound {
                                snippet: choices.snippet.unwrap(),
                                category: choices.category.unwrap(),
                                assign_as_expense: !choices.transfer.unwrap(),
                                assign_as_personal: match choices.sphere.unwrap() {
                                    Sphere::Personal => true,
                                    Sphere::Work => false,
                                },
                                require_confirmation: choices.require_confirmation.unwrap()
                            }
                        },
                        false => {
                            Classification::NoPatternOutbound {
                                assign_as_expense: !choices.transfer.unwrap(),
                                assign_as_personal: match choices.sphere.unwrap() {
                                    Sphere::Personal => true,
                                    Sphere::Work => false,
                                },
                                category: choices.category.unwrap(),
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
#[derive(PartialEq)]
pub enum Pattern {
    Inbound { snippet: String, category: String, assign_as_income: bool },
    Outbound {
        snippet: String,
        category: String,
        assign_as_expense: bool,
        assign_as_personal: bool,
        require_confirmation: bool
    }
}

#[ derive(Debug, PartialEq) ]
pub enum Classification<'a> {
    ExistingPattern(&'a Pattern, Option<PatternOverride>),
    NewPatternInbound {snippet: String, category: String, assign_as_income: bool},
    NewPatternOutbound {
        snippet: String,
        category: String,
        assign_as_expense: bool,
        assign_as_personal: bool,
        require_confirmation: bool
    },
    NoPatternInbound {category: String, assign_as_income: bool},
    NoPatternOutbound {category: String, assign_as_expense: bool, assign_as_personal: bool}
}

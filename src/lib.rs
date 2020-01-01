
pub mod report {
    use csv::StringRecord;
    use chrono::NaiveDate;
    use sha2::{Sha256, Digest};
    use rust_decimal::Decimal;
    use std::str::FromStr;

#[test]
    fn test_categorised_entry_instantiation() {
        let record = StringRecord::from(vec!["  20191101  ", "  foo bar  ", "  -191.60  ", "  0.00  "]);
        let ce = CategorisedEntry::new("personal", "bar cat", &record);

        assert_eq!(ce.category_type, "personal");
        assert_eq!(ce.category, "bar cat");
        assert_eq!(ce.date, NaiveDate::from_ymd(2019,11,1));
        assert_eq!(ce.description, "foo bar");
        assert_eq!(ce.expense, true);
        assert_eq!(ce.amount, Decimal::from_str("191.6").unwrap());
    }

#[test]
    fn test_categorised_entry_instantiation_with_income() {
        let record = StringRecord::from(vec!["20191101", "foo bar", "191.60", "0.00"]);
        let ce = CategorisedEntry::new("personal", "bar cat", &record);

        assert_eq!(ce.expense, false);
        assert_eq!(ce.amount, Decimal::from_str("191.6").unwrap());
    }

#[test]
    fn test_initialising_categorised_entry_creates_fingerprint() {
        let record = StringRecord::from(vec![" 20191101   ", "  foo bar  ", "  -191.60 ", "  0.00  "]);
        let ce = CategorisedEntry::new("personal", "bar cat", &record);

        let mut hasher = Sha256::new();
        hasher.input("20191101foo bar-191.600.00");
        let fingerprint = hex::encode(hasher.result());

        assert_eq!(ce.record_fingerprint, fingerprint);
    }

#[test]
    fn test_equality() {
        assert_eq!(
            CategorisedEntry {
                category_type: String::from("personal"),
                category: String::from("bar"),
                date: NaiveDate::from_ymd(2019,11,1),
                description: String::from("fuzzy"),
                expense: true,
                amount: Decimal::new(100,0),
                record_fingerprint: String::from("abc123")
            },
            CategorisedEntry {
                category_type: String::from("personal"),
                category: String::from("bar"),
                date: NaiveDate::from_ymd(2019,11,1),
                description: String::from("fuzzy"),
                expense: true,
                amount: Decimal::new(100,0),
                record_fingerprint: String::from("abc456")
            },
            );

        assert_ne!(
            CategorisedEntry {
                category_type: String::from("personal"),
                category: String::from("bar"),
                date: NaiveDate::from_ymd(2019,11,1),
                description: String::from("fuzzy"),
                expense: true,
                amount: Decimal::new(100,0),
                record_fingerprint: String::from("abc123")
            },
            CategorisedEntry {
                category_type: String::from("work"),
                category: String::from("bar"),
                date: NaiveDate::from_ymd(2019,11,1),
                description: String::from("fuzzy"),
                expense: true,
                amount: Decimal::new(100,0),
                record_fingerprint: String::from("abc123")
            },
            );

        assert_ne!(
            CategorisedEntry {
                category_type: String::from("personal"),
                category: String::from("bar"),
                date: NaiveDate::from_ymd(2019,11,1),
                description: String::from("fuzzy"),
                expense: true,
                amount: Decimal::new(100,0),
                record_fingerprint: String::from("abc123")
            },
            CategorisedEntry {
                category_type: String::from("personal"),
                category: String::from("baz"),
                date: NaiveDate::from_ymd(2019,11,1),
                description: String::from("fuzzy"),
                expense: true,
                amount: Decimal::new(100,0),
                record_fingerprint: String::from("abc123")
            },
            );

        assert_ne!(
            CategorisedEntry {
                category_type: String::from("personal"),
                category: String::from("bar"),
                date: NaiveDate::from_ymd(2019,11,2),
                description: String::from("fuzzy"),
                expense: true,
                amount: Decimal::new(100,0),
                record_fingerprint: String::from("abc123")
            },
            CategorisedEntry {
                category_type: String::from("work"),
                category: String::from("bar"),
                date: NaiveDate::from_ymd(2019,11,1),
                description: String::from("fuzzy"),
                expense: true,
                amount: Decimal::new(100,0),
                record_fingerprint: String::from("abc123")
            },
            );

        assert_ne!(
            CategorisedEntry {
                category_type: String::from("personal"),
                category: String::from("bar"),
                date: NaiveDate::from_ymd(2019,11,1),
                description: String::from("fizzy"),
                expense: true,
                amount: Decimal::new(100,0),
                record_fingerprint: String::from("abc123")
            },
            CategorisedEntry {
                category_type: String::from("work"),
                category: String::from("bar"),
                date: NaiveDate::from_ymd(2019,11,1),
                description: String::from("fuzzy"),
                expense: true,
                amount: Decimal::new(100,0),
                record_fingerprint: String::from("abc123")
            },
            );

        assert_ne!(
            CategorisedEntry {
                category_type: String::from("personal"),
                category: String::from("bar"),
                date: NaiveDate::from_ymd(2019,11,1),
                description: String::from("fuzzy"),
                expense: false,
                amount: Decimal::new(100,0),
                record_fingerprint: String::from("abc123")
            },
            CategorisedEntry {
                category_type: String::from("work"),
                category: String::from("bar"),
                date: NaiveDate::from_ymd(2019,11,1),
                description: String::from("fuzzy"),
                expense: true,
                amount: Decimal::new(100,0),
                record_fingerprint: String::from("abc123")
            },
            );

        assert_ne!(
            CategorisedEntry {
                category_type: String::from("personal"),
                category: String::from("bar"),
                date: NaiveDate::from_ymd(2019,11,1),
                description: String::from("fuzzy"),
                expense: true,
                amount: Decimal::new(100,0),
                record_fingerprint: String::from("abc123")
            },
            CategorisedEntry {
                category_type: String::from("work"),
                category: String::from("bar"),
                date: NaiveDate::from_ymd(2019,11,1),
                description: String::from("fuzzy"),
                expense: true,
                amount: Decimal::new(100,0),
                record_fingerprint: String::from("abc123")
            },
            );
    }

#[derive(Debug)]
    struct CategorisedEntry {
        category_type: String,
        category: String,
        date: NaiveDate,
        description: String,
        expense: bool,
        amount: Decimal,
        record_fingerprint: String
    }

    impl CategorisedEntry {
        fn new(category_type: &str, category: &str, original_record: &StringRecord) -> CategorisedEntry {

            let amount = original_record.get(2).unwrap().trim().parse::<f32>().unwrap();

            let mut hasher = Sha256::new();
            hasher.input(original_record.get(0).unwrap().trim());
            hasher.input(original_record.get(1).unwrap().trim());
            hasher.input(original_record.get(2).unwrap().trim());
            hasher.input(original_record.get(3).unwrap().trim());

            CategorisedEntry {
                category_type: String::from(category_type),
                category: String::from(category),
                date: NaiveDate::parse_from_str(original_record.get(0).unwrap().trim(), "%Y%m%d").unwrap(),
                description: String::from(original_record.get(1).unwrap().trim()),
                expense: amount < 0.0,
                amount: Decimal::from_str(&format!("{}", amount.abs())).unwrap(),
                record_fingerprint: hex::encode(hasher.result())
            }
        }
    }

    impl PartialEq for CategorisedEntry {
        fn eq(&self, other: &Self) -> bool {
            self.category_type == other.category_type &&
                self.category == other.category &&
                self.date == other.date &&
                self.description == other.description &&
                self.expense == other.expense && 
                self.amount == other.amount
        }
    }

#[test]
    fn test_activity_report_creation() {
        let report = ActivityReport::new();

        assert!(report.entries.is_empty())
    }

#[test]
    fn test_add_entry_to_activity_report() {
        let mut report = ActivityReport::new();
        let record = StringRecord::from(vec!["20191101", "foo bar", "-191.60", "0.00"]);
        let expected_entry = CategorisedEntry::new("personal", "bar cat", &record);

        report.add_entry("personal", "bar cat", &record);

        assert_eq!(report.entries, vec![expected_entry]);
    }

#[test]
    fn test_do_not_add_entries_that_map_to_the_same_record() {
        let mut report = ActivityReport::new();
        let record_1 = StringRecord::from(vec!["20191101", "foo bar", "-191.60", "0.00"]);
        let expected_entry_1 = CategorisedEntry::new("personal", "bar cat", &record_1);
        let record_2 = StringRecord::from(vec!["20191102", "fuzzy wuzzy", "111.11", "0.00"]);
        let expected_entry_2 = CategorisedEntry::new("work", "baz cat", &record_2);

        report.add_entry("personal", "bar cat", &record_1);
        report.add_entry("work", "baz cat", &record_2);
        report.add_entry("work", "foo cat", &record_1);

        assert_eq!(report.entries, vec![expected_entry_1, expected_entry_2]);
    }

#[test]
    fn test_total_entries() {
        let mut report = ActivityReport::new();
        let personal_expense_1 = StringRecord::from(vec!["20191101", "personal_expense_one", "-191.60", "0.00"]);
        let personal_expense_2 = StringRecord::from(vec!["20191101", "personal_expense_two", "-150.30", "0.00"]);
        let personal_income_1 = StringRecord::from(vec!["20191101", "personal_income_one", "500.30", "0.00"]);
        let personal_income_2 = StringRecord::from(vec!["20191101", "personal_income_two", "1000.90", "0.00"]);
        let work_expense_1 = StringRecord::from(vec!["20191101", "work_expense_one", "-300.10", "0.00"]);
        let work_expense_2 = StringRecord::from(vec!["20191101", "work_expense_two", "-400.70", "0.00"]);

        report.add_entry("personal", "bar cat", &personal_expense_1);
        report.add_entry("personal", "bar cat", &personal_expense_2);
        report.add_entry("personal", "bar cat", &personal_income_1);
        report.add_entry("personal", "bar cat", &personal_income_2);
        report.add_entry("work", "bar cat", &work_expense_1);
        report.add_entry("work", "bar cat", &work_expense_2);

        assert_eq!(report.total("work", true), Decimal::from_str("700.8").unwrap());
        assert_eq!(report.total("personal", true), Decimal::from_str("341.9").unwrap());
        assert_eq!(report.total("personal", false), Decimal::from_str("1501.2").unwrap());
    }

    pub struct ActivityReport {
        entries: Vec<CategorisedEntry>
    }

    impl ActivityReport {
        pub fn new() -> ActivityReport {
            ActivityReport { entries: Vec::new() }
        }

        pub fn add_entry(&mut self, category_type: &str, category: &str, original_record: &StringRecord) {
            let entry = CategorisedEntry::new(category_type, category, original_record);

            if !self.record_present(&entry) {
                self.entries.push(entry);
            } else {
                println!("Skipping entry - already present");
            }
        }

        pub fn total(&self, category_type: &str, expense: bool) -> Decimal {
            self.entries.iter().filter(|x| x.category_type == category_type && x.expense == expense).map(|x| x.amount).sum()
        }

        fn record_present(&self, entry: &CategorisedEntry) -> bool {
            let mut iter = self.entries.iter().filter(|e| e.record_fingerprint == entry.record_fingerprint);

            match iter.next() {
                Some(_) => true,
                None => false
            }
        }

    }
}

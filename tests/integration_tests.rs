use std::process::Command;
use csv::Writer;
use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use rexpect::session::spawn_command;
use rexpect::errors::*;

#[test]
fn test_happy_day_empty_config() {
    let home = assert_fs::TempDir::new().unwrap();
    let config_file = home.child(".bank_statement_importer.yml.new");
    let storage = assert_fs::TempDir::new().unwrap();

    setup_fixtures(&storage);
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.args(&[storage.path().to_str().unwrap(), "20191201"])
       .env("HOME", home.path().to_str().unwrap());

    let mut p = spawn_command(cmd, Some(500)).expect("Spawning command");

    for instruction in instructions() {
        println!("{}", instruction.entry);
        p.exp_regex(&instruction.entry).expect("Entry");
        p.exp_regex("Enter the existing category, or leave blank").expect("Prompt");
        if instruction.new_category {
            p.send_line("").expect("select new cat");
            p.exp_regex("New category:").expect("New cat prompt");
            p.send_line(&instruction.category).expect("new cat");
        } else {
            p.send_line(&instruction.category).expect("Existing cat");
        }

        p.exp_regex("Does this entry represent a transfer between accounts\\? \\[n/y\\]").expect("Transfer prompt");
        if instruction.transfer {
            p.send_line("y").expect("A transfer response");
        } else {
            p.send_line("n").expect("A non-transfer response");
        }

        match instruction.direction {
            Direction::Outbound => {
                if !instruction.transfer {
                    p.exp_regex("Is this a work or a personal entry \\[p/w\\]\\?").expect("Personal prompt");
                    if instruction.personal {
                        p.send_line("p").expect("Personal");
                    } else {
                        p.send_line("w").expect("Work");
                    }
                }
            },
            Direction::Inbound => ()
        }

        p.exp_regex("Would you like to create a pattern from this entry \\[y/n\\]\\?").expect("Create pattern prompt");
        if instruction.pattern {
            p.send_line("y").expect("Pattern");
            p.exp_regex("Please provide the snippet").expect("Snippet prompt");
            if let Some(snippet) = instruction.snippet {
                p.send_line(&snippet).expect("Snippet");
            }

            match instruction.direction {
                Direction::Outbound => {
                    if !instruction.transfer {
                        p.exp_regex("Should this always be assigned to personal \\[y/n\\]\\?")
                         .expect("Confirmation prompt");
                        if instruction.require_confirmation {
                            p.send_line("n").expect("Require confirmation");
                        } else {
                            p.send_line("y").expect("Do not require confirmation");
                        }
                    }
                },
                Direction::Inbound => ()
            }
        } else {
            p.send_line("n").expect("NO Pattern");
        }
    }

    // p.exp_regex("2019-12-01 outbound Fake Bookshop 333.33 0.00").expect("Record 1");
    // p.exp_regex("Enter the existing category, or leave blank").expect("Prompt");
    // p.send_line("").expect("new cat");
    // p.exp_regex("New category:").expect("New cat prompt");
    // p.send_line("books").expect("New books cat");
    // p.exp_regex("Does this entry represent a transfer between accounts\\? \\[n/y\\]").expect("Transfer prompt");
    // p.send_line("n").expect("Not a transfer");
    // p.exp_regex("Is this a work or a personal entry \\[p/w\\]\\?").expect("Work/personal prompt");
    // p.send_line("p").expect("Personal");
    // p.exp_regex("Would you like to create a pattern from this entry \\[y/n\\]\\?").expect("Create pattern prompt");
    // p.send_line("y").expect("Pattern");
    // p.exp_regex("Please provide the snippet").expect("Snippet prompt");
    // p.send_line("Fake Bookshop").expect("Snippet");
    // p.exp_regex("Should this always be assigned to personal \\[y/n\\]\\?").expect("Confirmation prompt");
    // p.send_line("n").expect("Confirmation response");
}

fn setup_fixtures(storage: &assert_fs::fixture::TempDir) {
    let dump_1 = storage.child("a.csv");
    let mut wtr_a = Writer::from_path(dump_1.path()).expect("Writer for file A");

    wtr_a.write_record(&["Date", "Description", "Amount", "Balance"]).expect("Writing record");
    wtr_a.write_record(&["20191130", "Insane purchase outside our date range", "-9999.99", "0.00"]).expect("Writing record");
    wtr_a.write_record(&["20191201", "Fake Bookshop", "-333.33", "0.00"]).expect("Writing record");
    wtr_a.write_record(&["20191202", "Salary", "555.55", "0.00"]).expect("Writing record");
    wtr_a.write_record(&["20200101", "Insane purchase outside our date range", "-9999.98", "0.00"]).expect("Writing record");
    wtr_a.flush().expect("Flushing");
}

#[derive(Debug)]
struct Instruction {
    entry: String,
    direction: Direction,
    new_category: bool,
    category: String,
    transfer: bool,
    personal: bool,
    pattern: bool,
    snippet: Option<String>,
    require_confirmation: bool
}

#[derive(Debug)]
enum Direction {
    Inbound, 
    Outbound
}

fn instructions() -> Vec<Instruction> {
    vec![
        Instruction {
            entry: "2019-12-01 outbound Fake Bookshop 333.33 0.00".into(),
            direction: Direction::Outbound,
            new_category: true,
            category: "books".into(),
            transfer: false,
            personal: true,
            pattern: true,
            snippet: Some("Fake Bookshop".into()),
            require_confirmation: true,
        },
        Instruction {
            entry: "2019-12-02 inbound Salary 555.55 0.00".into(),
            direction: Direction::Inbound,
            new_category: true,
            category: "salary".into(),
            transfer: false,
            personal: true,
            pattern: true,
            snippet: Some("Salary".into()),
            require_confirmation: false,
        },
    ]
}

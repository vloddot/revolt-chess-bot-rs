use super::Command;

#[derive(Default)]
pub struct Chess;

impl Command for Chess {
    fn get_name(&self) -> String {
        "chess".to_string()
    }

    fn get_aliases(&self) -> Vec<String> {
        vec!["play-chess".to_string()]
    }

    fn get_usage(&self) -> String {
        "<color> <opponent>".to_string()
    }
}

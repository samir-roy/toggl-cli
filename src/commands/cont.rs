use crate::api::client::ApiClient;
use crate::commands;
use crate::models;
use crate::picker;
use chrono::Utc;
use colored::Colorize;
use commands::stop::{StopCommand, StopCommandOrigin};
use models::{ResultWithDefaultError, TimeEntry};
use picker::{ItemPicker, PickableItem};

pub struct ContinueCommand;

impl ContinueCommand {
    pub async fn execute(
        api_client: impl ApiClient,
        picker: Option<Box<dyn ItemPicker>>,
    ) -> ResultWithDefaultError<()> {
        let running_time_entry =
            StopCommand::execute(&api_client, StopCommandOrigin::ContinueCommand).await?;

        // Wait 5 seconds to avoid rate limiting after stop command
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        let entities = api_client.get_entities().await?;
        if entities.time_entries.is_empty() {
            println!("{}", "No time entries in last 90 days".red());
            return Ok(());
        }

        let time_entry_to_continue = match picker {
            None => get_first_stopped_time_entry(entities.time_entries, running_time_entry),
            Some(time_entry_picker) => {
                let pickable_items = entities
                    .time_entries
                    .iter()
                    .map(|te| PickableItem::from_time_entry(te.clone()))
                    .collect();
                let picked_key = time_entry_picker.pick(pickable_items)?;
                let picked_time_entry = entities
                    .time_entries
                    .iter()
                    .find(|te| te.id == picked_key.id)
                    .unwrap();
                Some(picked_time_entry.clone())
            }
        };

        match time_entry_to_continue {
            None => println!("{}", "No time entry to continue".red()),
            Some(time_entry) => {
                let start_time = Utc::now();
                let time_entry_to_create = time_entry.as_running_time_entry(start_time);
                let continued_entry_id = api_client.create_time_entry(time_entry_to_create).await?;
                // Wait 5 seconds to avoid rate limiting before fetching updated entities
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                let entities = api_client.get_entities().await?;
                let continued_entry = entities
                    .time_entries
                    .iter()
                    .find(|te| te.id == continued_entry_id)
                    .unwrap();
                println!(
                    "{}\n{}",
                    "Time entry continued successfully".green(),
                    continued_entry
                )
            }
        }

        Ok(())
    }
}

fn get_first_stopped_time_entry(
    time_entries: Vec<TimeEntry>,
    running_time_entry: Option<TimeEntry>,
) -> Option<TimeEntry> {
    // Don't continue a running entry that was just stopped.
    let continue_entry_index = match running_time_entry {
        None => 0,
        Some(_) => 1,
    };
    time_entries.get(continue_entry_index).cloned()
}

/*
 Prod
 Copyright 2021-2025 Peter Pearson.
 Licensed under the Apache License, Version 2.0 (the "License");
 You may not use this file except in compliance with the License.
 You may obtain a copy of the License at
 http://www.apache.org/licenses/LICENSE-2.0
 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License.
 ---------
*/

use crate::params::{ParamValue, Params};

use super::control_actions::{ActionProvider, ActionError, ControlAction};
use super::control_common::ControlSession;

use super::terminal_helpers_linux;

use std::collections::BTreeMap;

// an attempt to make this code a bit more re-useable by other commands...
pub struct EditFileParams {
    filepath:       String,
    backup:         bool,

    replace_line_items:     Vec<ReplaceLineEntry>,
    insert_line_items:      Vec<InsertLineEntry>,
    comment_line_items:     Vec<CommentLineEntry>,
}

impl EditFileParams {
    pub fn new(filepath: &str, backup: bool) -> Self {
        EditFileParams { filepath: filepath.to_string(),
             backup,
             replace_line_items: Vec::new(),
             insert_line_items: Vec::new(),
             comment_line_items: Vec::new() }
    }

    pub fn set_all_items(mut self,
        replace_line_items: Vec<ReplaceLineEntry>,
        insert_line_items:  Vec<InsertLineEntry>,
        comment_line_items: Vec<CommentLineEntry>) -> Self {

        self.replace_line_items = replace_line_items;
        self.insert_line_items = insert_line_items;
        self.comment_line_items = comment_line_items;
        self
    }
}

// TODO: these two sets of enums/structs and functions have some duplication - see if we can reduce that...

#[derive(Clone, Debug, PartialEq)]
pub enum FileEditMatchType {
    Contains,
    Matches,
    StartsWith,
    EndsWith
}

// TODO: add the ability to have something which will keep any leading whitespace
//       when replacing, so alignment still matches?
pub struct ReplaceLineEntry {
    pub match_string:        String,
    pub replace_string:      String,
    pub report_failure:      bool,
    pub replaced:            bool,
    pub match_type:          FileEditMatchType,
}

impl ReplaceLineEntry {
    pub fn new(match_string: &str, replace_string: &str, report_failure: bool, match_type: FileEditMatchType) -> ReplaceLineEntry {
        ReplaceLineEntry { match_string: match_string.to_string(), replace_string: replace_string.to_string(),
             report_failure, replaced: false, match_type }
    }
}

fn extract_edit_line_entry_items<T>(params: &Params, key: &str, fun: &dyn Fn(&BTreeMap<String, ParamValue>
) -> Option<T>) -> Vec<T> {
    let mut line_entries = Vec::with_capacity(0);

    let param = params.get_raw_value(key);
    if let Some(ParamValue::Map(map)) = param {
        // cope with single items inline as map...
        if let Some(entry) = fun(map) {
            line_entries.push(entry);
        }
    }
    else if let Some(ParamValue::Array(array)) = param {
        // cope with multiple items of as an array, i.e. a map first-level item
        // of 'replaceLine', which itself is an array item with the multiple items
        // under it describing the params for each replacement. See the example
        // debian_edit_file2.yaml.
        for item in array {
            if let ParamValue::Map(map) = item {
                if let Some(entry) = fun(map) {
                    line_entries.push(entry);
                }
            }
        }
    }
    
    // alternatively, look for an 'items' value which might contain multiple line sub-entry items...
    let items = params.get_raw_value("items");
    if let Some(ParamValue::Array(array)) = items {
        for item in array {
            if let ParamValue::Map(map) = item {
                if let Some(ParamValue::Map(inner_params)) = map.get(key) {
                    if let Some(entry) = fun(inner_params) {
                        line_entries.push(entry);
                    }
                }
            }
        }
    }

    line_entries
}

fn get_edit_line_entry_match_type(entry: &BTreeMap<String, ParamValue>) -> FileEditMatchType {
    let match_type = match entry.get("matchType") {
        Some(ParamValue::Str(str)) => {
            match str.as_str() {
                "contains" => FileEditMatchType::Contains,
                "matches" => FileEditMatchType::Matches,
                "startsWith" => FileEditMatchType::StartsWith,
                "endsWith" => FileEditMatchType::EndsWith,
                _ => FileEditMatchType::Contains
            }
        },
        _ => FileEditMatchType::Contains
    };

    match_type
}

fn process_replace_line_entry(entry: &BTreeMap<String, ParamValue>) -> Option<ReplaceLineEntry> {
    let match_string = entry.get("matchString");
    let mut match_string_val = String::new();
    if let Some(ParamValue::Str(string)) = match_string {
        match_string_val = string.clone();
    }
    let replace_string = entry.get("replaceString");
    let mut replace_string_val = String::new();
    if let Some(ParamValue::Str(string)) = replace_string {
        replace_string_val = string.clone();
    }
    
    let match_type = get_edit_line_entry_match_type(entry);

    if !match_string_val.is_empty() && !replace_string_val.is_empty() {
        return Some(ReplaceLineEntry::new(&match_string_val, &replace_string_val, false, match_type));
    }

    None
}

#[derive(Clone, Debug, PartialEq)]
pub enum InsertLinePositionType {
    Above,
    Below,
}

pub struct InsertLineEntry {
    pub position_type:       InsertLinePositionType,
    pub match_string:        String,
    pub insert_string:       String,
    pub report_failure:      bool,
    pub replaced:            bool,
    pub match_type:          FileEditMatchType,
}

impl InsertLineEntry {
    pub fn new(position_type: InsertLinePositionType, match_string: &str, insert_string: &str, report_failure: bool,
        match_type: FileEditMatchType
    ) -> InsertLineEntry {
        InsertLineEntry { position_type, match_string: match_string.to_string(), insert_string: insert_string.to_string(),
             report_failure, replaced: false, match_type }
    }
}

fn process_insert_line_entry(entry: &BTreeMap<String, ParamValue>) -> Option<InsertLineEntry> {
    let match_string = entry.get("matchString");
    let mut match_string_val = String::new();
    if let Some(ParamValue::Str(string)) = match_string {
        match_string_val = string.clone();
    }
    let insert_string = entry.get("insertString");
    let mut insert_string_val = String::new();
    if let Some(ParamValue::Str(string)) = insert_string {
        insert_string_val = string.clone();
    }

    let position_type = match entry.get("position") {
        Some(ParamValue::Str(str)) => {
            match str.as_str() {
                "above" => InsertLinePositionType::Above,
                "below" => InsertLinePositionType::Below,
                _ => {
                    eprintln!("Warning: unrecognised 'position' value for insertLine entry item. Setting to 'below'.");
                    InsertLinePositionType::Below
                }
            }
        },
        _ => {
            eprintln!("Warning: undefined 'position' value for insertLine entry item. Setting to 'below'.");
            InsertLinePositionType::Below
        }
    };
    
    let match_type = get_edit_line_entry_match_type(entry);

    if !match_string_val.is_empty() && !insert_string_val.is_empty() {
        return Some(InsertLineEntry::new(position_type, &match_string_val, &insert_string_val, false, match_type));
    }

    None
}

pub struct CommentLineEntry {
    pub match_string:        String,
    pub comment_char:        String,
    pub report_failure:      bool,
    pub replaced:            bool,
    pub match_type:          FileEditMatchType,
}

impl CommentLineEntry {
    pub fn new(match_string: &str, comment_char: &str, report_failure: bool, match_type: FileEditMatchType) -> CommentLineEntry {
        CommentLineEntry { match_string: match_string.to_string(), comment_char: comment_char.to_string(),
             report_failure, replaced: false, match_type }
    }
}

fn process_comment_line_entry(entry: &BTreeMap<String, ParamValue>) -> Option<CommentLineEntry> {
    let match_string = entry.get("matchString");
    let mut match_string_val = String::new();
    if let Some(ParamValue::Str(string)) = match_string {
        match_string_val = string.clone();
    }
    let comment_char = entry.get("commentChar");
    let comment_char_val;
    if let Some(ParamValue::Str(string)) = comment_char {
        comment_char_val = string.clone();
    }
    else {
        // error...
        return None;
    }
    
    let match_type = get_edit_line_entry_match_type(entry);

    if !match_string_val.is_empty() && !comment_char_val.is_empty() {
        return Some(CommentLineEntry::new(&match_string_val, &comment_char_val, false, match_type));
    }

    None
}

// actual inner method which does most of the work, and in theory could be more re-useable in the future...
// TODO: this is pretty nasty and hacky, but works for all cases I want so far...
pub fn perform_edit_file_operation(action_provider: &dyn ActionProvider, connection: &mut ControlSession,
    edit_file_params: EditFileParams
) -> Result<(), ActionError> {

    if edit_file_params.backup {
        // TODO: something more robust than this...
        let mv_command = format!("cp {0} {0}.bak", edit_file_params.filepath);
        connection.conn.send_command(&action_provider.post_process_command(&mv_command));
        if let Some(strerr) = connection.conn.get_previous_stderr_response() {
            return Err(ActionError::FailedOther(format!("Error making backup copy of remote file path: {}", strerr)));
        }
    }

    // Note: the Stat returned by scp_recv() is currently a private field, so we can only access bits of it,
    //       so we need to do a full stat call remotely to get the actual info
    let stat_command = format!("stat {}", edit_file_params.filepath);
    connection.conn.send_command(&action_provider.post_process_command(&stat_command));
    if let Some(strerr) = connection.conn.get_previous_stderr_response() {
        return Err(ActionError::FailedOther(format!("Error accessing remote file path: {}", strerr)));
    }

    let stat_response = connection.conn.get_previous_stdout_response().to_string();
    // get the details from the stat call...
    let stat_details = terminal_helpers_linux::extract_details_from_stat_output(&stat_response);

    // download the file
    let string_contents = connection.conn.get_text_file_contents(&edit_file_params.filepath).unwrap();
    if string_contents.is_empty() {
        eprintln!("Error: remote file: {} has empty contents.", edit_file_params.filepath);
        return Err(ActionError::FailedOther("".to_string()));
    }
    let file_contents_lines = string_contents.lines();

    // brute force replacement - can optimise this or condense it, maybe both,
    // but just get it working for the moment...

    let item_matches_closure = |match_type : &FileEditMatchType, match_string: &str, line: &str| -> bool {
        if *match_type == FileEditMatchType::Contains {
            if line.contains(match_string) {
                return true;
            }
        }
        else if *match_type == FileEditMatchType::Matches {
            if line == match_string {
                return true;
            }
        }
        else if *match_type == FileEditMatchType::StartsWith {
            if line.starts_with(match_string) {
                return true;
            }
        }
        else if *match_type == FileEditMatchType::EndsWith {
            if line.ends_with(match_string) {
                return true;
            }
        }

        false
    };

    // this is disgusting, but can't be bothered with an enum...
    // TODO: also, this is going to be last-wins for any situation where multiple
    //       rules match a single line, but hopefully that won't happen for current use-cases...
    //       (clearly possible in theory though...)
    let mut insert_type = String::new();
    let mut insert_string = String::new();

    let mut new_file_contents_lines = Vec::new();
    for line in file_contents_lines {
        let mut have_processed_line = false;
        
        insert_type.clear();
        insert_string.clear();

        // TODO: it's ambiguous what we should do when both an insert item and replace item
        //       might match a line, but let's just ignore handling that situation for the moment,
        //       and assume params will be set exclusively for each, but this is obviously
        //       going to need re-thinking in the future and making more robust...

        for insert_item in &edit_file_params.insert_line_items {
            if item_matches_closure(&insert_item.match_type, &insert_item.match_string, line) {
                insert_type = match insert_item.position_type {
                    InsertLinePositionType::Above => "A".to_string(),
                    InsertLinePositionType::Below => "B".to_string()
                };
                insert_string = insert_item.insert_string.clone();
            }
        }

        for replace_item in &edit_file_params.replace_line_items {
            if item_matches_closure(&replace_item.match_type, &replace_item.match_string, line) {
                new_file_contents_lines.push(replace_item.replace_string.clone());
                have_processed_line = true;
            }
        }

        for comment_item in &edit_file_params.comment_line_items {
            if item_matches_closure(&comment_item.match_type, &comment_item.match_string, line) {
                new_file_contents_lines.push(format!("{}{}", comment_item.comment_char, line));
                have_processed_line = true;
            }
        }

        if !have_processed_line {
            // as mentioned above, on the assumption there won't currently be replace AND insert for a single line...
            if insert_type == "A" {
                new_file_contents_lines.push(insert_string.clone());
            }
            new_file_contents_lines.push(line.to_string());
            if insert_type == "B" {
                new_file_contents_lines.push(insert_string.clone());
            }
        }
    }

    // convert back to single string for entire file, and make sure we append a newline on the end...
    let new_file_contents_string = new_file_contents_lines.join("\n") + "\n";

    let mode;
    if let Some(stat_d) = stat_details {
        mode = i32::from_str_radix(&stat_d.0, 8).unwrap();
    }
    else {
        mode = 0o644;
        eprintln!("Can't extract stat details from file. Using 644 as default permissions mode.");
    }
    
    let send_res = connection.conn.send_text_file_contents(&edit_file_params.filepath, mode, &new_file_contents_string);
    if let Err(err) = send_res {
        return Err(ActionError::FailedOther(format!("Failed to send file contents back to host: {}", err)));
    }

    // TODO: change user and group of file to cached value from beforehand...

    Ok(())
}

pub fn edit_file(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    let filepath = action.get_required_string_param("filepath")?;
  
    let replace_line_items = extract_edit_line_entry_items(&action.params, "replaceLine", &process_replace_line_entry);
    let insert_line_items = extract_edit_line_entry_items(&action.params, "insertLine", &process_insert_line_entry);
    let comment_line_items = extract_edit_line_entry_items(&action.params, "commentLine", &process_comment_line_entry);
    if replace_line_items.is_empty() && insert_line_items.is_empty() && comment_line_items.is_empty() {
        eprintln!("Error: editFile Control Action had no items to perform...");
        return Err(ActionError::InvalidParams("".to_string()));
    }
    
    let edit_file_params = EditFileParams::new(&filepath, action.params.get_value_as_bool("backup").unwrap_or(false))
        .set_all_items(replace_line_items, insert_line_items, comment_line_items);

    perform_edit_file_operation(action_provider, connection, edit_file_params)
}

use crate::params::{ParamValue, Params};

use super::control_actions::{ActionProvider, ActionResult, ControlAction};
use super::control_common::{ControlSession};

use super::terminal_helpers_linux;

use std::collections::BTreeMap;
use std::path::Path;

pub fn generic_command(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
    if !action.params.has_value("command") {
        return ActionResult::InvalidParams("The 'command' parameter was not specified.".to_string());
    }

    let command = action.params.get_string_value("command").unwrap();
    if !command.is_empty() {
        connection.conn.send_command(&action_provider.post_process_command(&command));
    }

    // TODO: check if there's a 'errorIfStdErrOutputExists' param, and if so
    //       validate what the output of the command was...

    return ActionResult::Success;
}

pub fn create_directory(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
    // validate params
    if !action.params.has_value("path") {
        return ActionResult::InvalidParams("The 'path' parameter was not specified.".to_string());
    }

    let path_to_create = action.params.get_string_value("path").unwrap();

    // TODO: not sure about this... Maybe it should be called something else, maybe it should
    //       be the default?
    let multi_level = action.params.get_value_as_bool("multiLevel", false);
    let mkdir_command;
    if !multi_level {
        mkdir_command = format!("mkdir {}", path_to_create);
    }
    else {
        mkdir_command = format!("mkdir -p {}", path_to_create);
    }
    connection.conn.send_command(&action_provider.post_process_command(&mkdir_command));

    if let Some(permissions) = action.params.get_string_or_int_value_as_string("permissions") {
        let chmod_command = format!("chmod {} {}", permissions, path_to_create);
        connection.conn.send_command(&action_provider.post_process_command(&chmod_command));
    }

    if let Some(owner) = action.params.get_string_value("owner") {
        let chown_command = format!("chown {} {}", owner, path_to_create);
        connection.conn.send_command(&action_provider.post_process_command(&chown_command));
    }

    if let Some(group) = action.params.get_string_value("group") {
        let chgrp_command = format!("chgrp {} {}", group, path_to_create);
        connection.conn.send_command(&action_provider.post_process_command(&chgrp_command));
    }

    // TODO: check for 'groups' as well to handle setting multiple...

    return ActionResult::Success;
}

// infrastructure for edit_file()

// TODO: these two sets of enums/structs and functions have some duplication - see if we can reduce that...

#[derive(Clone, Debug, PartialEq)]
enum FileEditMatchType {
    Contains,
    Matches,
    StartsWith,
    EndsWith
}

struct ReplaceLineEntry {
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

fn extract_edit_line_entry_items<T>(params: &Params, key: &str, fun: &dyn Fn(&BTreeMap<String, ParamValue>) -> Option<T>) -> Vec<T> {
    let mut replace_line_entries = Vec::with_capacity(0);

    let param = params.get_raw_value(key);
    if let Some(ParamValue::Map(map)) = param {
        // cope with single items inline as map...
        if let Some(entry) = fun(&map) {
            replace_line_entries.push(entry);
        }
    }
    else if let Some(ParamValue::Array(array)) = param {
        // cope with multiple items as an array
        for item in array {
            if let ParamValue::Map(map) = item {
                if let Some(entry) = fun(&map) {
                    replace_line_entries.push(entry);
                }
            }
        }
    }

    return replace_line_entries;
}

fn get_replace_line_entry_match_type(entry: &BTreeMap<String, ParamValue>) -> FileEditMatchType {
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

    return match_type;
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
    
    let match_type = get_replace_line_entry_match_type(entry);

    if !match_string_val.is_empty() && !replace_string_val.is_empty() {
        return Some(ReplaceLineEntry::new(&match_string_val, &replace_string_val, false, match_type));
    }

    return None;
}

#[derive(Clone, Debug, PartialEq)]
enum InsertLinePositionType {
    Above,
    Below,
}

struct InsertLineEntry {
    pub position_type:       InsertLinePositionType,
    pub match_string:        String,
    pub insert_string:       String,
    pub report_failure:      bool,
    pub replaced:            bool,
    pub match_type:          FileEditMatchType,
}

impl InsertLineEntry {
    pub fn new(position_type: InsertLinePositionType, match_string: &str, insert_string: &str, report_failure: bool, match_type: FileEditMatchType) -> InsertLineEntry {
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
    
    let match_type = get_replace_line_entry_match_type(entry);

    if !match_string_val.is_empty() && !insert_string_val.is_empty() {
        return Some(InsertLineEntry::new(position_type, &match_string_val, &insert_string_val, false, match_type));
    }

    return None;
}

// TODO: this is pretty nasty and hacky, but works for all cases I want so far...
pub fn edit_file(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
    let filepath = action.params.get_string_value("filepath");
    if filepath.is_none() {
        return ActionResult::InvalidParams("The 'filepath' parameter was not specified.".to_string());
    }

    let replace_line_items = extract_edit_line_entry_items(&action.params, "replaceLine", &process_replace_line_entry);
    let insert_line_items = extract_edit_line_entry_items(&action.params, "insertLine", &process_insert_line_entry);
    if replace_line_items.is_empty() && insert_line_items.is_empty() {
        eprintln!("Error: editFile Control Action had no items to perform...");
        return ActionResult::InvalidParams("".to_string());
    }

    let filepath = filepath.unwrap();
    
    if action.params.get_value_as_bool("backup", false) {
        // TODO: something more robust than this...
        let mv_command = format!("cp {0} {0}.bak", filepath);
        connection.conn.send_command(&action_provider.post_process_command(&mv_command));
    }

    // Note: the Stat returned by scp_recv() is currently a private field, so we can only access bits of it,
    //       so we need to do a full stat call remotely to get the actual info
    let stat_command = format!("stat {}", filepath);
    connection.conn.send_command(&action_provider.post_process_command(&stat_command));

    let stat_response = connection.conn.get_previous_stdout_response().to_string();
    // get the details from the stat call...
    let stat_details = terminal_helpers_linux::extract_details_from_stat_output(&stat_response);

    // download the file
    let string_contents = connection.conn.get_text_file_contents(&filepath).unwrap();
    if string_contents.is_empty() {
        eprintln!("Error: remote file: {} has empty contents.", filepath);
        return ActionResult::Failed("".to_string());
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

        return false;
    };

    // this is disgusting, but can't be bothered with an enum...
    // TODO: also, this is going to be last-wins for any situation where multiple
    //       rules match a single line, but hopefully that won't happen for current use-cases...
    //       (clearly possible in theory though...)
    let mut insert_type = String::new();
    let mut insert_string = String::new();

    let mut new_file_contents_lines = Vec::new();
    for line in file_contents_lines {
        let mut have_replaced = false;
        
        insert_type.clear();
        insert_string.clear();

        // TODO: it's ambiguous what we should do when both an insert item and replace item
        //       might match a line, but let's just ignore handling that situation for the moment,
        //       and assume params will be set exclusively for each...

        for insert_item in &insert_line_items {
            if item_matches_closure(&insert_item.match_type, &insert_item.match_string, &line) {
                insert_type = match insert_item.position_type {
                    InsertLinePositionType::Above => "A".to_string(),
                    InsertLinePositionType::Below => "B".to_string()
                };
                insert_string = insert_item.insert_string.clone();
            }
        }

        for replace_item in &replace_line_items {
            if item_matches_closure(&replace_item.match_type, &replace_item.match_string, &line) {
                new_file_contents_lines.push(replace_item.replace_string.clone());
                have_replaced = true;
            }
        }

        if !have_replaced {
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
    
    let send_res = connection.conn.send_text_file_contents(&filepath, mode, &new_file_contents_string);
    if send_res.is_err() {
        return ActionResult::Failed("".to_string());
    }

    // TODO: change user and group of file to cached value from beforehand...

    return ActionResult::Success;
}

pub fn copy_path(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
    let source_path = action.params.get_string_value("sourcePath");
    if source_path.is_none() {
        return ActionResult::InvalidParams("The 'sourcePath' parameter was not specified.".to_string());
    }
    let source_path = source_path.unwrap();

    let dest_path = action.params.get_string_value("destPath");
    if dest_path.is_none() {
        return ActionResult::InvalidParams("The 'destPath' parameter was not specified.".to_string());
    }
    let dest_path = dest_path.unwrap();

    let recursive = action.params.get_value_as_bool("recursive", false);
    let update = action.params.get_value_as_bool("update", false);

    let mut option_flags = String::new();
    if recursive {
        option_flags.push_str("-R");
    }
    if update {
        option_flags.push_str(" -u");
    }
    option_flags = option_flags.trim().to_string();

    let cp_command = format!("cp {} {} {}", option_flags, source_path, dest_path);
    connection.conn.send_command(&action_provider.post_process_command(&cp_command));

    return ActionResult::Success;
}

pub fn download_file(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
    let source_url = action.params.get_string_value("sourceURL");
    if source_url.is_none() {
        return ActionResult::InvalidParams("The 'sourceURL' parameter was not specified.".to_string());
    }
    let source_url = source_url.unwrap();

    let dest_path = action.params.get_string_value("destPath");
    if dest_path.is_none() {
        return ActionResult::InvalidParams("The 'destPath' parameter was not specified.".to_string());
    }
    let dest_path = dest_path.unwrap();

    // use wget (maybe curl backup?) for the moment
    let wget_command = format!("wget {} -O {}", source_url, dest_path);
    connection.conn.send_command(&action_provider.post_process_command(&wget_command));

    if let Some(permissions) = action.params.get_string_or_int_value_as_string("permissions") {
        let chmod_command = format!("chmod {} {}", permissions, dest_path);
        connection.conn.send_command(&action_provider.post_process_command(&chmod_command));
    }

    if let Some(owner) = action.params.get_string_value("owner") {
        let chown_command = format!("chown {} {}", owner, dest_path);
        connection.conn.send_command(&action_provider.post_process_command(&chown_command));
    }

    if let Some(group) = action.params.get_string_value("group") {
        let chgrp_command = format!("chgrp {} {}", group, dest_path);
        connection.conn.send_command(&action_provider.post_process_command(&chgrp_command));
    }

    // see if we should also extract it
    if let Some(extract_dir) = action.params.get_string_value("extractDir") {
        // check this directory actually exists...
        if !extract_dir.is_empty()
        {
            let test_cmd = format!("test -d {} && echo \"yep\"", extract_dir);
            connection.conn.send_command(&action_provider.post_process_command(&test_cmd));

            // check the output is "yep"
            if connection.conn.get_previous_stdout_response().is_empty() {
                // doesn't exist...
                return ActionResult::Failed(format!("The 'extractDir' parameter directory: '{}' does not exist.", extract_dir));
            }

            // TODO: and check permissions?

            // now attempt to extract the file
            let tar_cmd = format!("tar -xf {} -C {}", dest_path, extract_dir);
            connection.conn.send_command(&action_provider.post_process_command(&tar_cmd));
        }
    }

    return ActionResult::Success;
}

pub fn transmit_file(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
    // local source path
    // TODO: not sure about this naming...
    let source_path = action.params.get_string_value("localSourcePath");
    if source_path.is_none() {
        return ActionResult::InvalidParams("The 'localSourcePath' parameter was not specified.".to_string());
    }
    let source_path = source_path.unwrap();

    // remote destination path
    let dest_path = action.params.get_string_value("remoteDestPath");
    if dest_path.is_none() {
        return ActionResult::InvalidParams("The 'remoteDestPath' parameter was not specified.".to_string());
    }
    let dest_path = dest_path.unwrap();

    let mut mode = 0o644;
    if let Some(permissions) = action.params.get_string_or_int_value_as_string("permissions") {
        mode = i32::from_str_radix(&permissions, 8).unwrap();
    }

    let send_res = connection.conn.send_file(&source_path, &dest_path, mode);
    if send_res.is_err() {
        return ActionResult::Failed("".to_string());
    }

    if let Some(owner) = action.params.get_string_value("owner") {
        let chown_command = format!("chown {} {}", owner, dest_path);
        connection.conn.send_command(&action_provider.post_process_command(&chown_command));
    }

    if let Some(group) = action.params.get_string_value("group") {
        let chgrp_command = format!("chgrp {} {}", group, dest_path);
        connection.conn.send_command(&action_provider.post_process_command(&chgrp_command));
    }

    // see if we should also extract it
    if let Some(extract_dir) = action.params.get_string_value("extractDir") {
        // check this directory actually exists...
        if !extract_dir.is_empty()
        {
            let test_cmd = format!("test -d {} && echo \"yep\"", extract_dir);
            connection.conn.send_command(&action_provider.post_process_command(&test_cmd));

            // check the output is "yep"
            if connection.conn.get_previous_stdout_response().is_empty() {
                // doesn't exist...
                return ActionResult::Failed(format!("The 'extractDir' parameter directory: '{}' does not exist.", extract_dir));
            }

            // TODO: and check permissions?

            // now attempt to extract the file
            let tar_cmd = format!("tar -xf {} -C {}", dest_path, extract_dir);
            connection.conn.send_command(&action_provider.post_process_command(&tar_cmd));
        }
    }

    return ActionResult::Success;
}

pub fn create_symlink(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
    let target_path = action.params.get_string_value("targetPath");
    if target_path.is_none() {
        return ActionResult::InvalidParams("The 'targetPath' parameter was not specified.".to_string());
    }
    let target_path = target_path.unwrap();

    // link name / path
    let link_path = action.params.get_string_value("linkPath");
    if link_path.is_none() {
        return ActionResult::InvalidParams("The 'linkPath' parameter was not specified.".to_string());
    }
    let link_path = link_path.unwrap();

    let link_dir = Path::new(&link_path);
    // TODO: error handling - and this might be a directory?
    let link_name = link_dir.file_name().unwrap();
    let link_dir = link_dir.parent().unwrap().to_str().unwrap();

    // Note: currently this will have to be true I think, otherwise we'd be creating the link
    //       in the current working directory which is likely to be unhelpful...
    if link_path.contains('/') {     
        let cd_command = format!("cd {}", link_dir);
        connection.conn.send_command(&action_provider.post_process_command(&cd_command));

        // check there was no error response
        if let Some(str) = connection.conn.get_previous_stderr_response() {
            eprintln!("create_symlink error: {}", str);
            return ActionResult::Failed(str.to_string());
        }
    }

    let ln_command = format!("ln -s {} {}", target_path, link_name.to_str().unwrap());
    connection.conn.send_command(&action_provider.post_process_command(&ln_command));

    if let Some(str) = connection.conn.get_previous_stderr_response() {
        eprintln!("create_symlink error: {}", str);
        return ActionResult::Failed(str.to_string());
    }

    return ActionResult::Success;
}

/*
 Prod
 Copyright 2021-2024 Peter Pearson.
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

use super::common_actions_unix_edit_file;

use super::control_actions::{ActionProvider, ActionError, ControlAction};
use super::control_common::ControlSession;

pub fn generic_command(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    let command = action.get_required_string_param("command")?;
    if !command.is_empty() {
        connection.conn.send_command(&action_provider.post_process_command(&command));
    }

    if action.params.get_value_as_bool("errorIfStdErrOutputExists", false) {
        if let Some(strerr) = connection.conn.get_previous_stderr_response() {
            return Err(ActionError::FailedCommand(format!("genericCommand action failed due to unexpected stderr output: {}", strerr)));
        }
    }

    if action.params.get_value_as_bool("errorIfNone0ExitCode", false) {
        if connection.conn.did_exit_with_error_code() {
            return Err(ActionError::FailedCommand(format!("genericCommand action failed due to none-0 exit code.")));
        }
    }

    // TODO: support for specifying expected string output to look for (stderr/stdout)
    //       for success criteria...

    // TODO: support for specifying expected number of lines of output as well...

    Ok(())
}

pub fn create_directory(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    let path_to_create = action.get_required_string_param("path")?;

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

    if connection.conn.did_exit_with_error_code() {
        return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&mkdir_command,
            action)));
    }

    if let Some(permissions) = action.params.get_string_or_int_value_as_string("permissions") {
        let chmod_command = format!("chmod {} {}", permissions, path_to_create);
        connection.conn.send_command(&action_provider.post_process_command(&chmod_command));

        if connection.conn.did_exit_with_error_code() {
            return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&chmod_command,
                action)));
        }
    }

    if let Some(owner) = action.params.get_string_value("owner") {
        let chown_command = format!("chown {} {}", owner, path_to_create);
        connection.conn.send_command(&action_provider.post_process_command(&chown_command));

        if connection.conn.did_exit_with_error_code() {
            return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&chown_command,
                action)));
        }
    }

    if let Some(group) = action.params.get_string_value("group") {
        let chgrp_command = format!("chgrp {} {}", group, path_to_create);
        connection.conn.send_command(&action_provider.post_process_command(&chgrp_command));

        if connection.conn.did_exit_with_error_code() {
            return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&chgrp_command,
                action)));
        }
    }

    // TODO: check for 'groups' as well to handle setting multiple...

    Ok(())
}

pub fn remove_directory(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    let path_to_remove = action.get_required_string_param("path")?;

    let recursive = action.params.get_value_as_bool("recursive", true);
    let rmdir_command;
    if !recursive {
        // TODO: Not really clear if this is worth it...
        rmdir_command = format!("rmdir {}", path_to_remove);
    }
    else {
        rmdir_command = format!("rm -rf {}", path_to_remove);
    }

    let ignore_failure = action.params.get_value_as_bool("ignoreFailure", false);

    connection.conn.send_command(&action_provider.post_process_command(&rmdir_command));
    if !ignore_failure && connection.conn.did_exit_with_error_code() {
        return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&rmdir_command,
            action)));
    }

    Ok(())
}

pub fn edit_file(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    return common_actions_unix_edit_file::edit_file(action_provider, connection, action);
}

pub fn copy_path(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    let source_path = action.get_required_string_param("sourcePath")?;
    let dest_path = action.get_required_string_param("destPath")?;
   
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

    if connection.conn.did_exit_with_error_code() {
        return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&cp_command,
            action)));
    }

    Ok(())
}

pub fn remove_file(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    let path = action.get_required_string_param("path")?;

    let rm_command = format!("rm {}", path);

    let ignore_failure = action.params.get_value_as_bool("ignoreFailure", false);

    connection.conn.send_command(&action_provider.post_process_command(&rm_command));
    if !ignore_failure && connection.conn.did_exit_with_error_code() {
        return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&rm_command,
            action)));
    }

    Ok(())
}

pub fn download_file(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    let source_url = action.get_required_string_param("sourceURL")?;
    let dest_path = action.get_required_string_param("destPath")?;

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
        if !extract_dir.is_empty() {
            let test_cmd = format!("test -d {} && echo \"yep\"", extract_dir);
            connection.conn.send_command(&action_provider.post_process_command(&test_cmd));

            // check the output is "yep"
            if connection.conn.get_previous_stdout_response().is_empty() {
                // doesn't exist...
                return Err(ActionError::FailedOther(format!("The 'extractDir' parameter directory: '{}' does not exist.", extract_dir)));
            }

            // TODO: and check permissions?

            // now attempt to extract the file, by attempting to work out the filename
            if dest_path.ends_with(".zip") {
                // assume it's a .zip file...
                let zip_cmd = format!("unzip {} -d {}", dest_path, extract_dir);
                connection.conn.send_command(&action_provider.post_process_command(&zip_cmd));
            }
            else {
                // otherwise, assume it's some form of tar file...
                let tar_cmd = format!("tar -xf {} -C {}", dest_path, extract_dir);
                connection.conn.send_command(&action_provider.post_process_command(&tar_cmd));
            }
        }
    }

    Ok(())
}

pub fn transmit_file(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    // local source path
    // TODO: not sure about this naming...
    let source_path = action.get_required_string_param("localSourcePath")?;
   
    // remote destination path
    let dest_path = action.get_required_string_param("remoteDestPath")?;

    let mut mode = 0o644;
    if let Some(permissions) = action.params.get_string_or_int_value_as_string("permissions") {
        mode = i32::from_str_radix(&permissions, 8).unwrap();
    }

    let send_res = connection.conn.send_file(&source_path, &dest_path, mode);
    if send_res.is_err() {
        return Err(ActionError::FailedOther("Failed to send file to host".to_string()));
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
        if !extract_dir.is_empty() {
            let test_cmd = format!("test -d {} && echo \"yep\"", extract_dir);
            connection.conn.send_command(&action_provider.post_process_command(&test_cmd));

            // check the output is "yep"
            if connection.conn.get_previous_stdout_response().is_empty() {
                // doesn't exist...
                return Err(ActionError::FailedOther(format!("The 'extractDir' parameter directory: '{}' does not exist.", extract_dir)));
            }

            // TODO: and check permissions?

            // now attempt to extract the file, by attempting to work out the filename
            if dest_path.ends_with(".zip") {
                // assume it's a .zip file...
                let zip_cmd = format!("unzip {} -d {}", dest_path, extract_dir);
                connection.conn.send_command(&action_provider.post_process_command(&zip_cmd));
            }
            else {
                // otherwise, assume it's some form of tar file...
                let tar_cmd = format!("tar -xf {} -C {}", dest_path, extract_dir);
                connection.conn.send_command(&action_provider.post_process_command(&tar_cmd));
            }

            // TODO: validate that extraction worked.
        }
    }

    Ok(())
}

pub fn receive_file(_action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    // remote source path
    // TODO: not sure about this naming...
    let source_path = action.get_required_string_param("remoteSourcePath")?;
 
    // local destination path
    let dest_path = action.get_required_string_param("localDestPath")?;

    let send_res = connection.conn.receive_file(&source_path, &dest_path);
    if send_res.is_err() {
        return Err(ActionError::FailedOther("Failed to receive file from host".to_string()));
    }

    Ok(())
}

pub fn create_symlink(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    let target_path = action.get_required_string_param("targetPath")?;
  
    // link path
    let link_path = action.get_required_string_param("linkPath")?;
    let ln_command = format!("ln -s {} {}", target_path, link_path);
    connection.conn.send_command(&action_provider.post_process_command(&ln_command));

    if connection.conn.did_exit_with_error_code() {
        return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&ln_command,
            action)));
    }

    Ok(())
}

pub fn create_file(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    let path_to_create = action.get_required_string_param("path")?;

    // TODO: maybe add support for creating any subdirs if required?
    
    // see if there's any content we need
    if let Some(content) = action.params.get_string_value("content") {
        // send the content as a file to write
        let send_res = connection.conn.send_text_file_contents(&path_to_create, 0o644, &content);
        if send_res.is_err() {
            return Err(ActionError::FailedOther("Failed to send text file contents to create file.".to_string()));
        }
    }
    else {
        // create an empty file, as there was no content param specified.
        let touch_command = format!("touch {}", path_to_create);
        connection.conn.send_command(&action_provider.post_process_command(&touch_command));
        if let Some(strerr) = connection.conn.get_previous_stderr_response() {
            return Err(ActionError::FailedOther(format!("Failed to create file: Err: {}", strerr)));
        }
    }

    // TODO: maybe move this somewhere more common, so it can be shared more?
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

    Ok(())
}
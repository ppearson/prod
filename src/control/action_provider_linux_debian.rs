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

use super::common_actions_linux;
use super::common_actions_unix;

use super::control_actions::{ActionProvider, ActionError, ControlAction, GenericError, SystemDetailsResult};
use super::control_common::{ControlSession, ControlSessionParams};
use super::terminal_helpers_linux;

pub struct AProviderLinuxDebian {
    // params which give us some hints as to context of session, i.e. username - sudo vs root, etc.
    session_params: ControlSessionParams,
}

impl AProviderLinuxDebian {
    pub fn new(session_params: ControlSessionParams) -> AProviderLinuxDebian {
        AProviderLinuxDebian { session_params }
    }

    pub fn name() -> String {
        "linux_debian".to_string()
    }
}

impl ActionProvider for AProviderLinuxDebian {
    fn name(&self) -> String {
        AProviderLinuxDebian::name()
    }

    fn get_session_params(&self) -> Option<&ControlSessionParams> {
        Some(&self.session_params)
    }

    fn generic_command(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::generic_command(self, connection, action)
    }

    // this is not really an Action, as it doesn't modify anything, it just returns values, but...
    fn get_system_details(&self, connection: &mut ControlSession) -> Result<SystemDetailsResult, GenericError> {
        common_actions_linux::get_system_details(self, connection)
    }

    fn add_user(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_linux::add_user(self, connection, action)
    }

    fn create_directory(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::create_directory(self, connection, action)
    }

    fn remove_directory(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::remove_directory(self, connection, action)
    }

    fn install_packages(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        // use apt-get, because the commands for that will apparently be much more stable, compared to apt
        // which might change as it's designed to be more user-facing...

        let packages;
        if let Some(package) = action.params.get_string_value("package") {
            // single package for convenience...
            packages = vec![package];
        }
        else if action.params.has_value("packages") {
            packages = action.params.get_values_as_vec_of_strings("packages");
        }
        else {
            return Err(ActionError::InvalidParams("No 'package' string parameter or 'packages' string array parameter were specified.".to_string()));
        }

        if packages.is_empty() {
            return Err(ActionError::InvalidParams("The resulting 'packages' list was empty.".to_string()));
        }

        // with some providers (Vultr), apt-get runs automatically just after the instance first starts,
        // so we can't run apt-get manually, as the lock file is locked, so wait until apt-get has stopped running
        // by default... 
        let wait_for_apt_get_lockfile = action.params.get_value_as_bool("waitForPMToFinish").unwrap_or(true);
        
        // by default, update the list of packages, as with some Debian images (i.e. Linode's),
        // this needs to be done first, otherwise no packages can't be found...
        let update_packages = action.params.get_value_as_bool("update").unwrap_or(true);

        let apt_get_install_params = AptGetInstallParams::new(wait_for_apt_get_lockfile, update_packages)
            .add_packages(packages);

        // do the actual core work...
        self.perform_apt_package_install(&apt_get_install_params, connection)
    }

    fn remove_packages(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        // use apt-get, because the commands for that will apparently be much more stable, compared to apt
        // which might change as it's designed to be more user-facing...

        let packages_string;
        if let Some(package) = action.params.get_string_value("package") {
            // single package for convenience...
            packages_string = package;
        }
        else if action.params.has_value("packages") {
            let packages = action.params.get_values_as_vec_of_strings("packages");
            packages_string = packages.join(" ");
        }
        else {
            return Err(ActionError::InvalidParams("No 'package' string parameter or 'packages' string array parameter were specified.".to_string()));
        }

        if packages_string.is_empty() {
            return Err(ActionError::InvalidParams("The resulting 'packages' string list was empty.".to_string()));
        }

        // with some providers (Vultr), apt-get runs automatically just after the instance first starts,
        // so we can't run apt-get manually, as the lock file is locked, so wait until apt-get has stopped running
        // by default... 
        let wait_for_apt_get_lockfile = action.params.get_value_as_bool("waitForPMToFinish").unwrap_or(true);
        if wait_for_apt_get_lockfile {
            let mut try_count = 0;
            while try_count < 20 {
                connection.conn.send_command(&self.post_process_command("pidof apt-get"));

                if !connection.conn.had_command_response() {
                    // it's likely no longer running, so we can continue...
                    break;
                }

                // TODO: only print this once eventually, but might be useful like this for the moment...
                println!("Waiting for existing apt-get to finish before removing packages...");

                // sleep a bit to give things a chance...
                std::thread::sleep(std::time::Duration::from_secs(20));

                try_count += 1;
            }
        }

        let apt_get_command = format!("export DEBIAN_FRONTEND=noninteractive; apt-get -y remove {}", packages_string);
        connection.conn.send_command(&self.post_process_command(&apt_get_command));

        let ignore_failure = action.params.get_value_as_bool("ignoreFailure").unwrap_or(false);

        if connection.conn.did_exit_with_error_code() {
            if !ignore_failure {
                return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&apt_get_command,
                    action)));
            }
        }

        Ok(())
    }

    fn systemctrl(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_linux::systemctrl(self, connection, action)
    }

    fn firewall(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        // debian doesn't need ufw firewall enabled first before adding rules
        common_actions_linux::firewall(self, connection, action, false)
    }

    fn edit_file(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::edit_file(self, connection, action)
    }

    fn copy_path(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::copy_path(self, connection, action)
    }

    fn remove_file(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::remove_file(self, connection, action)
    }

    fn download_file(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::download_file(self, connection, action)
    }

    fn transmit_file(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::transmit_file(self, connection, action)
    }

    fn receive_file(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::receive_file(self, connection, action)
    }

    fn create_symlink(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::create_symlink(self, connection, action)
    }

    fn set_time_zone(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_linux::set_time_zone(self, connection, action)
    }

    fn disable_swap(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_linux::disable_swap(self, connection, action)
    }

    fn create_file(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::create_file(self, connection, action)
    }

    fn add_group(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_linux::add_group(self, connection, action)
    }

    fn set_hostname(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_linux::set_hostname(self, connection, action)
    }

    fn create_systemd_service(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_linux::create_systemd_service(self, connection, action)
    }

    fn configure_ssh(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {
        common_actions_unix::configure_ssh(self, connection, action)
    }

    fn add_package_repo(&self, connection: &mut ControlSession, action: &ControlAction) -> Result<(), ActionError> {

        // check that we have required params first
        let repo_type = action.get_required_string_param("type")?;

        // for the moment, we only support "manualURL", but we can support more
        // in the future for things like PPAs and such with add-apt-repository
        // as alternative types...
        if repo_type == "manualURL" {
            // we're going to manually add new key and source list definition files,
            // based on content downloaded from URLs..

            // get required params for this type...
            let key_url = action.get_required_string_param("keyURL")?;
            let source_list_def_url = action.get_required_string_param("sourceListDefURL")?;
            let local_file_prefix = action.get_required_string_param("localFilePrefix")?;
            
            // first of all, because this is somewhat manual (although it's apparently how things should be done these days
            // now that apt-key is deprecated, although it's possible we might be able to use add-apt-repository?), we need
            // to make sure some packages are installed...
            let required_packages = ["gpg", "debian-keyring", "debian-archive-keyring", "apt-transport-https", "curl"];

            // TODO: see if there's a way we can automatically and robustly de-duplicate multiple package_install actions
            //       which are doing this so only the first one waits and performs an update...?
            let apt_get_install_params = AptGetInstallParams::new(true, true)
                .add_packages(required_packages.iter().map(|i| i.to_string()).collect());

            // do the initial core work installing those pre-requisite packages - this will 
            // return an error automatically if it fails
            self.perform_apt_package_install(&apt_get_install_params, connection)?;

            // by default, pass in '--yes' to get gpg to overwrite key files which already exist (so it doesn't throw up an
            // interactive warning), but allow overriding whether to do that
            let fail_on_existing_key_file = action.params.get_value_as_bool("failOnExistingKeyFile").unwrap_or(false);
            let overwrite_existing = if !fail_on_existing_key_file { "--yes " } else { "" };

            // now that those are installed, download, decrypt and install the .key file
            let key_install_cmd = format!("curl -1sLf '{}' | gpg {}--dearmor -o /usr/share/keyrings/{}-archive-keyring.gpg",
                key_url, overwrite_existing, local_file_prefix);

            connection.conn.send_command(&self.post_process_command(&key_install_cmd));

            // TODO: might have to chmod it to 644 in some future cases (sudo?)?

            // this won't have printed anything to stdout/stderr regardless of whether it failed or succeeded, but it
            // should only have an exit code of 0 if it succeeded, so check for that...
            if connection.conn.did_exit_with_error_code() {
                return Err(ActionError::FailedCommand(format!("key install command failed with error exit code. command: {}", key_install_cmd)));
            }

            // now try and download the source list definition and install that
            let source_list_def_install_cmd = format!("curl -1sLf '{}' | tee /etc/apt/sources.list.d/{}.list",
                source_list_def_url, local_file_prefix);
            
            connection.conn.send_command(&self.post_process_command(&source_list_def_install_cmd));

            // again, this command is not amazingly robust to detecting validate issues with, although it does output
            // the contents of the file if downloaded, but it prints nothing if it fails.
            // The exit code is not useful either in this case, but if it fails the target filename
            // will have a size of 0 bytes, so do a stat and check the file size to try and work
            // out if it succeeded or not...
            
            let stat_command = format!("stat /etc/apt/sources.list.d/{}.list", local_file_prefix);
            connection.conn.send_command(&self.post_process_command(&stat_command));
            if let Some(strerr) = connection.conn.get_previous_stderr_response() {
                return Err(ActionError::FailedOther(format!("Error accessing remote file path: {}", strerr)));
            }
        
            let stat_response = connection.conn.get_previous_stdout_response().to_string();
            // get the details from the stat call...
            let stat_details = terminal_helpers_linux::extract_details_from_stat_output(&stat_response);
            if stat_details.is_none() {
                // assume the file didn't get created, but this is unlikely to happen, even if something
                // does go wrong (much more likely is a 0-byte file is created)
                return Err(ActionError::FailedCommand(
                    format!("repo package source list install command failed: {}",
                    source_list_def_install_cmd)));
            }
            // now check that the filesize is > 0
            let stat_details = stat_details.unwrap();
            if stat_details.file_size == 0 {
                return Err(ActionError::FailedCommand(
                    format!("could not download the package source list to: /etc/apt/sources.list.d/{}.list",
                    local_file_prefix)));
            }
        }
        else {
            return Err(ActionError::InvalidParams("The 'type' parameter did not contain a currently-supported value type. Only 'manualURL' is currently supported.".to_string()));
        }

        // hopefully that all worked okay, so now all we have to do is reload the packages with the new source added.
        let update_packages = action.params.get_value_as_bool("updatePackages").unwrap_or(true);
        if update_packages {
            let apt_get_command = "apt-get -y update".to_string();
            connection.conn.send_command(&self.post_process_command(&apt_get_command));
        }
        
        Ok(())
    }
}

struct AptGetInstallParams {
    // whether to wait for the package manager (apt-get) to finish if it's running already.
    // on some VPS images (i.e. Vultr), after a VM is provisioned and started, it runs
    // apt-get update first, and in that case, we can't also run it, as it has a file lock.
    wait_for_pm_to_finish:  bool,

    // whether to update the packages list first with an 'update' command.
    // Some Debian images (i.e. from Linode) need this to be done first, otherwise they
    // never find any packages with the apt-get install command.
    update_packages_list:   bool,

    packages_to_install:    Vec<String>,
}

impl AptGetInstallParams {
    fn new(wait_for_pm_to_finish: bool,
        update_packages_list: bool) -> AptGetInstallParams {
        AptGetInstallParams { wait_for_pm_to_finish,
            update_packages_list,
            packages_to_install: Vec::new() }
    }

    fn add_packages(mut self, packages: Vec<String>) -> Self {
        self.packages_to_install.extend(packages);
        self
    }
}

// internal re-useable function which does the core work, in order to be easily call-able from other
// actions
impl AProviderLinuxDebian {
    fn perform_apt_package_install(&self, params: &AptGetInstallParams,
        connection: &mut ControlSession) -> Result<(), ActionError> {

        // use apt-get, because the commands for that will apparently be much more stable, compared to apt
        // which might change as it's designed to be more user-facing...
    
        let packages_string = params.packages_to_install.join(" ");
    
        // with some providers (Vultr), apt-get runs automatically just after the instance first starts,
        // so we can't run apt-get manually, as the lock file is locked, so wait until apt-get has stopped running
        // by default... 
        let wait_for_apt_get_lockfile = params.wait_for_pm_to_finish;
        if wait_for_apt_get_lockfile {
            let mut try_count = 0;
            while try_count < 20 {
                connection.conn.send_command(&self.post_process_command("pidof apt-get"));
    
                if !connection.conn.had_command_response() {
                    // it's likely no longer running, so we can continue...
                    break;
                }
    
                // TODO: only print this once eventually, but might be useful like this for the moment...
                println!("Waiting for existing apt-get to finish before installing packages...");
    
                // sleep a bit to give things a chance...
                std::thread::sleep(std::time::Duration::from_secs(20));
    
                try_count += 1;
            }
        }
    
        // unattended-upgr /// ??!
    
        // TODO: might be worth polling for locks on /var/lib/dpkg/lock-frontend ?
    
        // by default, update the list of packages, as with some Debian images (i.e. Linode's),
        // this needs to be done first, otherwise no packages can't be found...
        let update_packages = params.update_packages_list;
        if update_packages {
            let apt_get_command = "apt-get -y update".to_string();
            connection.conn.send_command(&self.post_process_command(&apt_get_command));
        }
    
        // Note: first time around, unless we export this DEBIAN_FRONTEND env variable, we get a
        //       debconf / dpkg-preconfigure issue with $TERM apparently being unset, and complaints
        //       about the frontend not being useable. Interestingly, without setting the env variable,
        //       trying again after the first failure works, and it's not time-dependent...
    
        let apt_get_command = format!("export DEBIAN_FRONTEND=noninteractive; apt-get -y install {}", packages_string);
        connection.conn.send_command(&self.post_process_command(&apt_get_command));
    
        if connection.conn.did_exit_with_error_code() {
            return Err(ActionError::FailedCommand(
                format!("Unexpected error exit code after running command: '{}' trying to install Debian packages.",
                apt_get_command)));
        }
    
        Ok(())
    }
}


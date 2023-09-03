/*
 Prod
 Copyright 2021-2023 Peter Pearson.
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

use std::collections::BTreeSet;

use super::provision_common::{ProvisionActionType, ProvisionActionResult};
use super::provision_manager::ListType;
use super::provision_params::ProvisionParams;

pub trait ProvisionProvider {

    // not sure about this one - ideally it'd be static, but...
    fn name(&self) -> String {
        return "".to_string();
    }

    fn supports_interactive(&self) -> bool {
        return false;
    }

    fn prompt_interactive(&self) -> Vec<(String, String)> {
        return Vec::new();
    }

    fn configure_interactive(&mut self) -> bool {
        return false;
    }

    fn configure(&mut self) -> bool {
        return false;
    }

    fn is_configured(&self) -> bool {
        return false;
    }

    // actual API items

    fn list_available(&self, _list_type: ListType) -> bool {
        return true;
    }

    fn get_required_params_for_action(&self, _action: ProvisionActionType) -> BTreeSet<&str> {
        return BTreeSet::new();
    }

    fn create_instance(&self, _params: &ProvisionParams, _dry_run: bool) -> ProvisionActionResult {
        return ProvisionActionResult::NotSupported;
    }

    fn delete_instance(&self, _params: &ProvisionParams, _dry_run: bool) -> ProvisionActionResult {
        return ProvisionActionResult::NotSupported;
    }

}

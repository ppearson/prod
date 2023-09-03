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

// Note: This implementation is very much *not* complete, and is really just a placeholder,
//       and may well be removed in the future...

use ureq;
//use ureq::Error;
//use serde_json::{Value};

use std::collections::BTreeSet;

use crate::provision::provision_provider::ProvisionProvider;
use crate::provision::provision_common::{ProvisionActionType, ProvisionActionResult};
use crate::provision::provision_manager::ListType;
use crate::provision::provision_params::ProvisionParams;

pub struct ProviderOpenStack {
    openstack_endpoint_uri:  String,
    openstack_username:      String,
    openstack_api_token:     String,
    openstack_tenant_name:   String,
}

impl ProviderOpenStack {
    pub fn new() -> ProviderOpenStack {
        ProviderOpenStack { openstack_endpoint_uri: String::new(), openstack_username: String::new(),
                            openstack_tenant_name: String::new(), openstack_api_token: String::new() }
    }
}

impl ProvisionProvider for ProviderOpenStack {
    fn name(&self) -> String {
        return "openstack".to_string();
    }

    fn supports_interactive(&self) -> bool {
        return false;
    }

    fn prompt_interactive(&self) -> Vec<(String, String)> {
        let items = Vec::new();
        return items;
    }

    fn configure_interactive(&mut self) -> bool {
        return false;
    }

    fn configure(&mut self) -> bool {
        let endpoint_uri = std::env::var("PROD_OS_ENDPOINT_URI");
        if let Err(_e) = endpoint_uri {
            return false;
        }
        let endpoint_uri = endpoint_uri.unwrap().trim().to_string();
        self.openstack_endpoint_uri = endpoint_uri;

        let username = std::env::var("PROD_OS_USERNAME");
        if let Err(_e) = username {
            return false;
        }
        self.openstack_username = username.unwrap();

        let api_token = std::env::var("PROD_OS_API_TOKEN");
        if let Err(_e) = api_token {
            return false;
        }

        self.openstack_api_token = api_token.unwrap();

        let tenant_name = std::env::var("PROD_OS_TENANT_NAME");
        if let Err(_e) = tenant_name {
            return false;
        }

        self.openstack_tenant_name = tenant_name.unwrap();

        return true;
    }

    fn is_configured(&self) -> bool {
        return !self.openstack_endpoint_uri.is_empty();
    }

    // actual commands

    fn list_available(&self, list_type: ListType) -> bool {
        let url = match list_type {
            ListType::Plans => "/flavors",
            ListType::Regions => "/os-availability-zone",
            ListType::OSs => "images",
            _ => {
                return false;
            }
        };

        let full_url = format!("{}{}", self.openstack_endpoint_uri, url);

        let resp = ureq::get(&full_url)
//            .set("Authorization", &format!("Bearer {}", self.linode_api_key))
            .call();
        
        if resp.is_err() {
            eprintln!("Error querying OpenStack API: {} for list request: {:?}", self.openstack_endpoint_uri, resp.err());
            return false;
        }

        let resp_string = resp.unwrap().into_string().unwrap();

        // TODO: format these nicely, and maybe filter them?...

        println!("{}", resp_string);
        
        return true;
    }

    fn get_required_params_for_action(&self, action: ProvisionActionType) -> BTreeSet<&str> {
        let mut params = BTreeSet::new();
        if action == ProvisionActionType::CreateInstance {
            params.insert("name");
            params.insert("availability_zone");
            params.insert("flavorRef");
            params.insert("imageRef");
        }
        params
    }

    fn create_instance(&self, _params: &ProvisionParams, _dry_run: bool) -> ProvisionActionResult {
        // let name_str = params.get_string_value("name", "");
        // let availability_zone_str = params.get_string_value("availability_zone", "");
        // let flavor_ref_str = params.get_string_value("flavorRef", "");
        // let image_ref_str = params.get_string_value("imageRef", "");


        return ProvisionActionResult::Failed("".to_string());
    }
}

impl ProviderOpenStack {
    
}

#![allow(clippy::ptr_arg, dead_code, clippy::enum_variant_names)]

use std::collections::{BTreeSet, HashMap};

use google_apis_common as common;
use tokio::time::sleep;

/// A scope is needed when requesting an
/// [authorization token](https://developers.google.com/workspace/drive/labels/guides/authorize).
#[derive(PartialEq, Eq, Ord, PartialOrd, Hash, Debug, Clone, Copy)]
pub enum Scope {
    /// View, use, and manage Drive labels.
    DriveLabels,

    /// View and use Drive labels.
    DriveLabelsReadonly,

    /// View, edit, create, and delete all Drive labels in your organization,
    /// and view your organization's label-related administration policies.
    DriveLabelsAdmin,

    /// View all Drive labels and label-related administration policies in your
    /// organization.
    DriveLabelsAdminReadonly,
}

impl AsRef<str> for Scope {
    fn as_ref(&self) -> &str {
        match *self {
            Scope::DriveLabels => "https://www.googleapis.com/auth/drive.labels",
            Scope::DriveLabelsReadonly => "https://www.googleapis.com/auth/drive.labels.readonly",
            Scope::DriveLabelsAdmin => "https://www.googleapis.com/auth/drive.admin.labels",
            Scope::DriveLabelsAdminReadonly => {
                "https://www.googleapis.com/auth/drive.admin.labels.readonly"
            }
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for Scope {
    fn default() -> Scope {
        Scope::DriveLabelsReadonly
    }
}

#[derive(Clone)]
pub struct DriveLabelsHub<C> {
    pub client: common::Client<C>,
    pub auth: Box<dyn common::GetToken>,
    _user_agent: String,
    _base_url: String,
}

impl<C> common::Hub for DriveLabelsHub<C> {}

impl<'a, C> DriveLabelsHub<C> {
    pub fn new<A: 'static + common::GetToken>(
        client: common::Client<C>,
        auth: A,
    ) -> DriveLabelsHub<C> {
        DriveLabelsHub {
            client,
            auth: Box::new(auth),
            _user_agent: "google-api-rust-client/6.0.0".to_string(),
            _base_url: "https://drivelabels.googleapis.com/".to_string(),
        }
    }

    pub fn labels(&'a self) -> LabelMethods<'a, C> {
        LabelMethods { hub: self }
    }

    /// Set the user-agent header field to use in all requests to the server.
    /// It defaults to `google-api-rust-client/6.0.0`.
    ///
    /// Returns the previously set user-agent.
    pub fn user_agent(&mut self, agent_name: String) -> String {
        std::mem::replace(&mut self._user_agent, agent_name)
    }

    /// Set the base url to use in all requests to the server.
    /// It defaults to `https://www.googleapis.com/drive/v3/`.
    ///
    /// Returns the previously set base url.
    pub fn base_url(&mut self, new_base_url: String) -> String {
        std::mem::replace(&mut self._base_url, new_base_url)
    }
}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde_with::serde_as]
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Label {
    #[serde(rename = "name")]
    pub name: Option<String>,
    #[serde(rename = "id")]
    pub id: Option<String>,
    #[serde(rename = "revisionId")]
    pub revision_id: Option<String>,
    #[serde(rename = "labelType")]
    pub label_type: Option<String>,
    #[serde(rename = "creator")]
    pub creator: Option<User>,
    #[serde(rename = "createTime")]
    pub create_time: Option<String>,
    #[serde(rename = "revisionCreator")]
    pub revision_creator: Option<User>,
    #[serde(rename = "revisionCreateTime")]
    pub revision_create_time: Option<String>,
    #[serde(rename = "publisher")]
    pub publisher: Option<User>,
    #[serde(rename = "publishTime")]
    pub publish_time: Option<String>,
    #[serde(rename = "disabler")]
    pub disabler: Option<User>,
    #[serde(rename = "disableTime")]
    pub disable_time: Option<String>,
    #[serde(rename = "customer")]
    pub customer: Option<String>,
    pub properties: Option<LabelProperty>,
    pub fields: Option<Vec<Field>>,
    // We ignore the remaining fields.
}

impl common::Part for Label {}

impl common::ResponseResult for Label {}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde_with::serde_as]
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LabelProperty {
    pub title: Option<String>,
    pub description: Option<String>,
}

impl common::Part for LabelProperty {}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde_with::serde_as]
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Field {
    id: Option<String>,
    #[serde(rename = "queryKey")]
    query_key: Option<String>,
    properties: Option<FieldProperty>,
    #[serde(rename = "selectionOptions")]
    selection_options: Option<SelectionOption>,
}

impl common::Part for Field {}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde_with::serde_as]
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FieldProperty {
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub required: Option<bool>,
}

impl common::Part for FieldProperty {}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde_with::serde_as]
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SelectionOption {
    #[serde(rename = "listOptions")]
    pub list_options: Option<String>,
    pub choices: Option<Vec<Choice>>,
}

impl common::Part for SelectionOption {}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde_with::serde_as]
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Choice {
    id: Option<String>,
    properties: Option<ChoiceProperties>,
    // We ignore the remaining fields.
}

impl common::Part for Choice {}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde_with::serde_as]
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ChoiceProperties {
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    description: Option<String>,
}

impl common::Part for ChoiceProperties {}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde_with::serde_as]
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LabelList {
    pub labels: Option<Vec<Label>>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

impl common::ResponseResult for LabelList {}

/// Information about a Drive user.
///
/// This type is not used in any activity, and only used as *part* of another schema.
///
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde_with::serde_as]
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct User {
    /// Output only. A plain text displayable name for this user.
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    /// Output only. The email address of the user. This may not be present in certain contexts if the user has not made their email address visible to the requester.
    #[serde(rename = "emailAddress")]
    pub email_address: Option<String>,
    /// Output only. Identifies what kind of resource this is. Value: the fixed string `"drive#user"`.
    pub kind: Option<String>,
    /// Output only. Whether this user is the requesting user.
    pub me: Option<bool>,
    /// Output only. The user's ID as visible in Permission resources.
    #[serde(rename = "permissionId")]
    pub permission_id: Option<String>,
    /// Output only. A link to the user's profile photo, if available.
    #[serde(rename = "photoLink")]
    pub photo_link: Option<String>,
}

impl common::Part for User {}

pub struct LabelMethods<'a, C>
where
    C: 'a,
{
    hub: &'a DriveLabelsHub<C>,
}

impl<C> common::MethodsBuilder for LabelMethods<'_, C> {}

impl<'a, C> LabelMethods<'a, C> {
    /// Create a builder to help you perform the following tasks:
    ///
    /// List labels
    pub fn list(&self) -> LabelListCall<'a, C> {
        LabelListCall {
            hub: self.hub,
            _delegate: Default::default(),
            _additional_params: Default::default(),
            _scopes: Default::default(),
        }
    }
}

/// Lists the workspace's labels.
pub struct LabelListCall<'a, C>
where
    C: 'a,
{
    hub: &'a DriveLabelsHub<C>,
    _delegate: Option<&'a mut dyn common::Delegate>,
    _additional_params: HashMap<String, String>,
    _scopes: BTreeSet<String>,
}

impl<C> common::CallBuilder for LabelListCall<'_, C> {}

impl<'a, C> LabelListCall<'a, C>
where
    C: common::Connector,
{
    /// Perform the operation you have built so far.
    pub async fn doit(mut self) -> common::Result<(common::Response, LabelList)> {
        use common::url::Params;
        use hyper::header::{AUTHORIZATION, CONTENT_LENGTH, USER_AGENT};

        let mut dd = common::DefaultDelegate;
        let dlg: &mut dyn common::Delegate = self._delegate.unwrap_or(&mut dd);
        dlg.begin(common::MethodInfo {
            id: "drivelabels.labels.list",
            http_method: hyper::Method::GET,
        });

        for &field in ["alt"].iter() {
            if self._additional_params.contains_key(field) {
                dlg.finished(false);
                return Err(common::Error::FieldClash(field));
            }
        }

        // TODO: We don't handle any of the query params.
        let mut params = Params::with_capacity(2 + self._additional_params.len());

        params.extend(self._additional_params.iter());

        params.push("alt", "json");
        let url = self.hub._base_url.clone() + "v2/labels";

        if self._scopes.is_empty() {
            self._scopes
                .insert(Scope::DriveLabelsReadonly.as_ref().to_string());
        }

        let url = params.parse_with_url(&url);

        loop {
            let token = match self
                .hub
                .auth
                .get_token(&self._scopes.iter().map(String::as_str).collect::<Vec<_>>()[..])
                .await
            {
                Ok(token) => token,
                Err(e) => match dlg.token(e) {
                    Ok(token) => token,
                    Err(e) => {
                        dlg.finished(false);
                        return Err(common::Error::MissingToken(e));
                    }
                },
            };
            let req_result = {
                let client = &self.hub.client;
                dlg.pre_request();
                let mut req_builder = hyper::Request::builder()
                    .method(hyper::Method::GET)
                    .uri(url.as_str())
                    .header(USER_AGENT, self.hub._user_agent.clone());

                if let Some(token) = token.as_ref() {
                    req_builder = req_builder.header(AUTHORIZATION, format!("Bearer {}", token));
                }

                let request = req_builder
                    .header(CONTENT_LENGTH, 0_u64)
                    .body(common::to_body::<String>(None));
                client.request(request.unwrap()).await
            };

            match req_result {
                Err(err) => {
                    if let common::Retry::After(d) = dlg.http_error(&err) {
                        sleep(d).await;
                        continue;
                    }
                    dlg.finished(false);
                    return Err(common::Error::HttpError(err));
                }
                Ok(res) => {
                    let (parts, body) = res.into_parts();
                    let body = common::Body::new(body);
                    if !parts.status.is_success() {
                        let bytes = common::to_bytes(body).await.unwrap_or_default();
                        let error = serde_json::from_str(&common::to_string(&bytes));
                        let response = common::to_response(parts, bytes.into());

                        if let common::Retry::After(d) =
                            dlg.http_failure(&response, error.as_ref().ok())
                        {
                            sleep(d).await;
                            continue;
                        }

                        dlg.finished(false);

                        return Err(match error {
                            Ok(value) => common::Error::BadRequest(value),
                            _ => common::Error::Failure(response),
                        });
                    }
                    let response = {
                        let bytes = common::to_bytes(body).await.unwrap_or_default();
                        let encoded = common::to_string(&bytes);
                        match serde_json::from_str(&encoded) {
                            Ok(decoded) => (common::to_response(parts, bytes.into()), decoded),
                            Err(error) => {
                                dlg.response_json_decode_error(&encoded, &error);
                                return Err(common::Error::JsonDecodeError(
                                    encoded.to_string(),
                                    error,
                                ));
                            }
                        }
                    };

                    dlg.finished(true);
                    return Ok(response);
                }
            }
        }
    }

    /// The delegate implementation is consulted whenever there is an intermediate result, or if something goes wrong
    /// while executing the actual API request.
    ///
    /// ````text
    ///                   It should be used to handle progress information, and to implement a certain level of resilience.
    /// ````
    ///
    /// Sets the *delegate* property to the given value.
    pub fn delegate(mut self, new_value: &'a mut dyn common::Delegate) -> LabelListCall<'a, C> {
        self._delegate = Some(new_value);
        self
    }

    /// Set any additional parameter of the query string used in the request.
    /// It should be used to set parameters which are not yet available through their own
    /// setters.
    ///
    /// Please note that this method must not be used to set any of the known parameters
    /// which have their own setter method. If done anyway, the request will fail.
    ///
    /// # Additional Parameters
    ///
    /// * *$.xgafv* (query-string) - V1 error format.
    /// * *access_token* (query-string) - OAuth access token.
    /// * *alt* (query-string) - Data format for response.
    /// * *callback* (query-string) - JSONP
    /// * *fields* (query-string) - Selector specifying which fields to include in a partial response.
    /// * *key* (query-string) - API key. Your API key identifies your project and provides you with API access, quota, and reports. Required unless you provide an OAuth 2.0 token.
    /// * *oauth_token* (query-string) - OAuth 2.0 token for the current user.
    /// * *prettyPrint* (query-boolean) - Returns response with indentations and line breaks.
    /// * *quotaUser* (query-string) - Available to use for quota purposes for server-side applications. Can be any arbitrary string assigned to a user, but should not exceed 40 characters.
    /// * *uploadType* (query-string) - Legacy upload protocol for media (e.g. "media", "multipart").
    /// * *upload_protocol* (query-string) - Upload protocol for media (e.g. "raw", "multipart").
    pub fn param<T>(mut self, name: T, value: T) -> LabelListCall<'a, C>
    where
        T: AsRef<str>,
    {
        self._additional_params
            .insert(name.as_ref().to_string(), value.as_ref().to_string());
        self
    }

    /// Identifies the authorization scope for the method you are building.
    ///
    /// Use this method to actively specify which scope should be used, instead of the default [`Scope`] variant
    /// [`Scope::DriveLabelsReadonly`].
    ///
    /// The `scope` will be added to a set of scopes. This is important as one can maintain access
    /// tokens for more than one scope.
    ///
    /// Usually there is more than one suitable scope to authorize an operation, some of which may
    /// encompass more rights than others. For example, for listing resources, a *read-only* scope will be
    /// sufficient, a read-write scope will do as well.
    pub fn add_scope<St>(mut self, scope: St) -> LabelListCall<'a, C>
    where
        St: AsRef<str>,
    {
        self._scopes.insert(String::from(scope.as_ref()));
        self
    }
    /// Identifies the authorization scope(s) for the method you are building.
    ///
    /// See [`Self::add_scope()`] for details.
    pub fn add_scopes<I, St>(mut self, scopes: I) -> LabelListCall<'a, C>
    where
        I: IntoIterator<Item = St>,
        St: AsRef<str>,
    {
        self._scopes
            .extend(scopes.into_iter().map(|s| String::from(s.as_ref())));
        self
    }

    /// Removes all scopes, and no default scope will be used either.
    /// In this case, you have to specify your API-key using the `key` parameter (see [`Self::param()`]
    /// for details).
    pub fn clear_scopes(mut self) -> LabelListCall<'a, C> {
        self._scopes.clear();
        self
    }
}

use anyhow::Error;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use url::form_urlencoded::byte_serialize;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{History, Location, PopStateEvent};
use yew::format::{Json, Nothing};
use yew::prelude::*;
use yew::services::console::ConsoleService;
use yew::services::fetch::{FetchService, FetchTask, Request, Response, Uri};
use yew::utils::document;

//https://github.com/JaniM/variant-go-server/blob/4f7b8206f605887a1d0e6bb5a10b6d4ae895e4dd/client/src/utils.rs#L30
#[macro_export]
macro_rules! if_html {
    (let $pat:pat = $cond:expr => $($body:tt)+) => {
        if let $pat = $cond {
            html!($($body)+)
        } else {
            html!()
        }
    };
    ($cond:expr => $($body:tt)+) => {
        if $cond {
            html!($($body)+)
        } else {
            html!()
        }
    };
}

pub struct App {
    link: ComponentLink<Self>,
    search: String,
    show_hash: String,
    settings_click: i64,
    port: String,
    fetching: bool,
    network_task: Option<yew::services::fetch::FetchTask>,
}

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct SystemSettings {
    pub port: String,
    pub ignore_domains: Vec<String>,
    pub ignore_strings: Vec<String>,
    pub indexer_enabled: bool,
}

pub struct Settings {
    link: ComponentLink<Self>,
    settings: Option<SystemSettings>,
    port: String,
    new_ignore_string: String,
    new_ignore_domains: String,
    fetching: bool,
    network_task: Option<yew::services::fetch::FetchTask>,
}

impl Settings {
    fn post_json(
        &mut self,
        binary: bool,
        url: String,
        body: &SystemSettings,
        stored_data: String,
    ) -> yew::services::fetch::FetchTask {
        let callback = self
            .link
            .callback(move |response: Response<Json<Result<Value, Error>>>| {
                let (meta, Json(data)) = response.into_parts();
                if meta.status.is_success() {
                    Msg::FetchReady((stored_data.clone(), data))
                } else {
                    Msg::Ignore // FIXME: Handle this error accordingly.
                }
            });
        let request = Request::post(url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(Json(body))
            .unwrap();
        if binary {
            FetchService::fetch_binary(request, callback).unwrap()
        } else {
            FetchService::fetch(request, callback).unwrap()
        }
    }

    fn update_settings(&mut self, port: Option<String>) {
        self.fetching = true;
        let settings = self.settings.clone();
        let settings = settings.unwrap();

        self.network_task = Some(self.post_json(
            false,
            format!(
                "http://localhost:{}/settings",
                port.unwrap_or_else(|| self.port.clone()),
            ),
            &settings,
            "settings".to_string(),
        ));
    }
    fn fetch_json(
        &mut self,
        binary: bool,
        url: String,
        stored_data: String,
    ) -> yew::services::fetch::FetchTask {
        let callback = self
            .link
            .callback(move |response: Response<Json<Result<Value, Error>>>| {
                let (meta, Json(data)) = response.into_parts();
                if meta.status.is_success() {
                    Msg::FetchReady((stored_data.clone(), data))
                } else {
                    Msg::Ignore // FIXME: Handle this error accordingly.
                }
            });
        let request = Request::get(url)
            .header("Accept", "application/json")
            .body(Nothing)
            .unwrap();
        if binary {
            FetchService::fetch_binary(request, callback).unwrap()
        } else {
            FetchService::fetch(request, callback).unwrap()
        }
    }
    fn fetch_settings(&mut self, port: Option<String>) {
        self.fetching = true;
        self.network_task = Some(self.fetch_json(
            false,
            format!(
                "http://localhost:{}/settings",
                port.unwrap_or_else(|| self.port.clone())
            ),
            "settings".to_string(),
        ));
    }

    fn chip_it(&self, chip: &str) -> Html {
        let domain = chip.clone();
        let domain = domain.to_string();
        let id = format!("chip-{}", domain);
        ConsoleService::log(&format!("{:?}", domain));
        html! {
          <div class="chip" key=id.clone() id=id>
            { format!("{}", domain.clone()) }
            <i class="close material-icons" onclick=self.link.callback(move |e: MouseEvent| Msg::RemoveIgnoreDomains(domain.clone()))>{"close"}</i>
          </div>
        }
    }

    fn loading_html(&self) -> Html {
        if_html!(self.fetching =>
        <div class="progress">
            <div class="indeterminate"></div>
        </div>
        )
    }
    fn loaded(&self) -> Html {
        if let Some(settings) = self.settings.as_ref() {
            ConsoleService::log(&format!("{:?}", settings));
            html! {<>
              <div class="row">
                <div class="cliplist">
                { settings.ignore_domains.iter().map(|d| self.chip_it(d)).collect::<Html>() }
                </div>
                <div class="input-field col s12">
                  { "The url will not be indexed if it matches any of this list. Only html content types are indexed. Space adds the value to the list." }
                  <input id="ignore_domains" type="text" value=self.new_ignore_domains.clone() oninput=self.link.callback(|e: InputData| Msg::UpdateIgnoreDomains(e.value))/>
                </div>
                <div class="switch">
                    { "Indexer: " }
                    <label>
                      { "Off" }
                      <input type="checkbox" checked=settings.indexer_enabled, onclick=self.link.callback(|_| Msg::ToggleIndexer ) />
                      <span class="lever"></span>
                      { "On" }
                    </label>
                </div>
              </div>
            </>}
        } else {
            html! {
              <div class="row">
              { "The settings from the server have not loaded yet. If you changed the default port please manually restart the server and reload the page" }
              </div>
            }
        }
    }
    fn settings_modal(&self) -> Html {
        html! {
        <div id="setting_modal" class="modal">
          <div class="modal-content">
              <div class="row">
                <form class="col s12">
                  { self.loaded()}
                  <div class="row">
                    <div class="input-field col s6">
                      <input id="port" type="text" value={self.settings.as_ref().and_then(|s| Some(s.port.clone())).unwrap_or_else(|| self.port.clone())} oninput=self.link.callback(|e: InputData| Msg::UpdatePort(e.value))/>
                      <label class="active" for="port">{ "Server Port (manual restarts required)" }</label>
                    </div>
                  </div>
                </form>
              </div>
          </div>
        </div>
        }
    }
}

#[derive(Properties, Clone, PartialEq, Debug)]
pub struct SettingsProp {
    pub clicked_at: i64,
}
impl Component for Settings {
    type Message = Msg;
    type Properties = SettingsProp;

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut s = Settings {
            link,
            settings: None,
            new_ignore_domains: String::new(),
            new_ignore_string: String::new(),
            port: "7172".to_string(),
            fetching: false,
            network_task: None,
        };
        s.fetch_settings(Some(s.port.clone()));
        s
    }
    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.fetch_settings(Some(self.port.clone()));
        true
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::UpdateIgnoreDomains(string) => {
                if string.ends_with(' ') {
                    if let Some(settings) = self.settings.as_mut() {
                        settings.ignore_domains.push(string.trim().to_string());
                        self.update_settings(None);
                    }
                    self.new_ignore_domains = String::new();
                } else {
                    self.new_ignore_domains = string;
                }
            }

            Msg::RemoveIgnoreDomains(string) => {
                if let Some(settings) = self.settings.as_mut() {
                    settings.ignore_domains.retain(|x| x != &string);
                    self.update_settings(None);
                }
            }

            Msg::ToggleIndexer => {
                if let Some(settings) = self.settings.as_mut() {
                    settings.indexer_enabled = !settings.indexer_enabled;
                    self.update_settings(None);
                }
            }

            Msg::UpdatePort(string) => {
                // server needs to be pre configured
                self.port = string;
                self.fetch_settings(Some(self.port.clone()));
            }
            Msg::FetchReady(response) => match response.0.as_str() {
                "settings" => {
                    self.fetching = false;
                    self.network_task = None;
                    if let Ok(results) = response.1 {
                        let results: Option<SystemSettings> = serde_json::from_value(results).ok();
                        ConsoleService::log(&format!("{:?}", results));
                        self.settings = results;
                    }
                }
                _ => {}
            },
            _ => {}
        }
        true
    }

    fn view(&self) -> Html {
        html! {
        <>
            { self.settings_modal() }
        </>
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct SearchArray {
    results: Vec<SearchJson>,
    meta: Option<SearchMeta>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SearchMeta {
    document_count: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SearchJson {
    id: String,
    title: String,
    url: String,
    summary: String,
    description: String,
    added_at: String,
    last_accessed_at: String,
    keywords: Vec<String>,
    tags: Vec<String>,
    bookmarked: i64,
    pinned: i64,
    duplicate: i64,
    accessed_count: i64,
}

pub struct SearchResults {
    search_json: Option<SearchArray>,
    link: ComponentLink<Self>,
    search: String,
    new_tag: String,
    port: String,
    queued_search: Option<String>,
    fetching: bool,
    props: SearchProps,
    network_task: Option<yew::services::fetch::FetchTask>,
    pin_task: Option<yew::services::fetch::FetchTask>,
}
#[derive(Properties, Clone, PartialEq, Debug)]
pub struct SearchProps {
    search_input: String,
}
impl Component for SearchResults {
    type Message = Msg;
    type Properties = SearchProps;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let _empty: Vec<serde_json::Result<Request<Vec<u8>>>> = vec![];

        let mut s = SearchResults {
            link,
            search_json: None,
            search: props.search_input.clone(),
            // write /read from local stoage
            // https://dev.to/davidedelpapa/yew-tutorial-04-and-services-for-all-1non
            port: "7172".to_string(),
            queued_search: None,
            new_tag: String::new(),
            fetching: false,
            network_task: None,
            pin_task: None,
            props,
        };
        if !s.search.is_empty() {
            s.update(Msg::Search(s.search.clone()));
        }
        s
    }
    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        ConsoleService::log(&format!("p{:?}", props));
        if self.props != props {
            self.update(Msg::Search(props.search_input.clone()));
            self.props = props;
            true
        } else {
            false
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Pin(string) => self.remote_set_attribute(&string, &"pinned", 1),
            Msg::Unpin(string) => self.remote_set_attribute(&string, &"pinned", 0),
            Msg::Hide(string) => self.remote_set_attribute(&string, &"hide", 1),
            Msg::HideDomain(string) => self.remote_set_attribute(&string, &"hide_domain", 1),
            Msg::UpdatePort(string) => {
                self.port = string;
            }
            Msg::Untag(change) => {
                let (url, tag) = change;
                self.remote_set_tag(&url, &tag, "remove");
                self.new_tag = String::new();
            }
            Msg::Tag(change) => {
                let (url, tag) = change;
                if tag.ends_with(' ') {
                    self.remote_set_tag(&url, &tag, "add");
                    self.new_tag = String::new();
                } else {
                    self.new_tag = tag;
                }
            }
            Msg::Search(search_string) => {
                self.search = search_string;
                // remove dup?
                if !self.search.trim().is_empty() {
                    if self.fetching {
                        //wonky debounce.
                        self.queued_search = Some(self.search.clone());
                    } else {
                        self.fetch_search(&self.search.clone())
                    }
                } else {
                    self.search_json = None;
                    self.fetching = false;
                    self.queued_search = None;
                    self.network_task = None;
                }
            }
            Msg::FetchReady(response) => {
                if let Some(next) = &self.queued_search {
                    self.fetching = false;
                    self.network_task = None;
                    self.fetch_search(&next.clone())
                } else {
                    match response.0.as_str() {
                        "search_items" => {
                            self.fetching = false;
                            self.network_task = None;
                            let window = web_sys::window().unwrap();
                            if let Ok(history) = window.history() {
                                /*
                                if history
                                    .push_state_with_url(
                                        &JsValue::from_str(""),
                                        "",
                                        Some(&format!("/?q={}", self.search)),
                                    )
                                    .is_err()
                                {
                                    ConsoleService::log("Set history is not working");
                                }*/
                            }
                            let results = response.1.ok();
                            ConsoleService::log(&format!("{:?}", results));
                            // remove dup
                            self.for_value(results);
                        }
                        "set_attributes" => {
                            self.pin_task = None;

                            let results = response.1.ok();
                            ConsoleService::log(&format!("{:?}", results));
                            // just reload the search.
                            self.update(Msg::Search(self.search.clone()));
                        }
                        _ => {}
                    }
                }

                self.queued_search = None;
            }
            _ => {}
        }
        true
    }

    fn view(&self) -> Html {
        html! {
        <>
            { self.search_results() }
        </>
        }
    }
}

impl SearchResults {
    fn for_value(&mut self, results: Option<Value>) {
        match results {
            Some(results) => {
                self.search_json = serde_json::from_value(results).ok();
            }
            None => {}
        }
    }

    fn fetch_search(&mut self, string: &str) {
        self.fetching = true;
        let urlencoded: String = byte_serialize(string.as_bytes()).collect();
        // cause "debounce" the js kills the request the server still processes them
        self.network_task = Some(self.fetch_json(
            false,
            format!("http://localhost:{}/search?q={}", self.port, urlencoded),
            "search_items".to_string(),
        ));
    }

    fn remote_set_tag(&mut self, url: &str, tag: &str, action: &str) {
        let urlencoded: String = byte_serialize(url.as_bytes()).collect();
        let urlencoded_tag: String = byte_serialize(tag.as_bytes()).collect();
        // cause "debounce" the js kills the request the server still processes them
        self.pin_task = Some(self.fetch_json(
            false,
            format!(
                "http://localhost:{}/attributes_array?url={}&field=tag&value={}&action={}",
                self.port, urlencoded, urlencoded_tag, action
            ),
            "set_attributes".to_string(),
        ));
    }

    fn remote_set_attribute(&mut self, url: &str, field: &str, value: i64) {
        let urlencoded: String = byte_serialize(url.as_bytes()).collect();
        // cause "debounce" the js kills the request the server still processes them
        self.pin_task = Some(self.fetch_json(
            false,
            format!(
                "http://localhost:{}/attributes?url={}&field={}&value={}",
                self.port, urlencoded, field, value
            ),
            "set_attributes".to_string(),
        ));
    }
    fn fetch_json(
        &mut self,
        binary: bool,
        url: String,
        stored_data: String,
    ) -> yew::services::fetch::FetchTask {
        let callback = self
            .link
            .callback(move |response: Response<Json<Result<Value, Error>>>| {
                let (meta, Json(data)) = response.into_parts();
                if meta.status.is_success() {
                    Msg::FetchReady((stored_data.clone(), data))
                } else {
                    Msg::Ignore // FIXME: Handle this error accordingly.
                }
            });
        let request = Request::get(url)
            .header("Accept", "application/json")
            .body(Nothing)
            .unwrap();
        if binary {
            FetchService::fetch_binary(request, callback).unwrap()
        } else {
            FetchService::fetch(request, callback).unwrap()
        }
    }

    fn loading_html(&self) -> Html {
        if_html!(self.fetching =>
        <div class="progress">
            <div class="indeterminate"></div>
        </div>
        )
    }

    fn search_item_html(&self, obj: &SearchJson) -> Html {
        let link = if let Some(location) = document().location() {
            if let Ok(href) = location.origin() {
                format!(
                    "{}/{}?view={}",
                    href,
                    location.pathname().unwrap_or("".to_string()),
                    obj.id
                )
            } else {
                format!("/index.html?view={}", obj.id)
            }
        } else {
            format!("/index.html?view={}", obj.id)
        };
        html! {
          <li class="collection-item avatar">
            <span class="title"><a href=link.clone() target="_blank">{&obj.title}{" "}{&obj.url}</a></span>
            <p> {&obj.description} <br/>
            {&obj.summary}
            <br/>
            { obj.tags.iter().map(|keyword| self.chip(&obj.url.clone(), &keyword)).collect::<Vec<Html>>()}
            </p>

            { self.pinned(&obj.pinned, obj.url.clone()) }
            { self.bookmarked(&obj.bookmarked) }
            { self.menu(&obj.url, &obj.id) }
          </li>
        }
    }

    fn menu(&self, url: &str, id: &str) -> Html {
        let base_url = url.clone();
        let base_url = base_url.to_string();

        let base2_url = url.clone();
        let base2_url = base2_url.to_string();

        let base3_url = url.clone();
        let base3_url = base3_url.to_string();
        html! {
            <>
                <a class="dropdown-trigger secondary-content" href="#" data-target=format!("dropdown-{}",id)><i class="material-icons">{"arrow_drop_down"}</i> </a>
                <ul id=format!("dropdown-{}",id) class="dropdown-content">
                    <li class="clickclose"><a href="#!" onclick=self.link.callback(move |e| Msg::Hide(base_url.clone())) >{"hide url"}</a></li>
                    <li class="clickclose"><a href="#!" onclick=self.link.callback(move |e| Msg::HideDomain(base2_url.clone())) >{"hide domain"}</a></li>
                    <li> <input id="add_tag" type="text" placeholder="Add Tag" value=self.new_tag.clone()
                    oninput=self.link.callback(move |e: InputData| Msg::Tag((base3_url.clone(), e.value)))/> </li>
                </ul>
            </>
        }
    }

    fn pinned(&self, marked: &i64, url: String) -> Html {
        let url_pin = url.clone();
        if marked == &1 {
            html! {
            <a href="#!" class="secondary-content tooltipped search-pinned"
                data-position="bottom"
                data-url=url
                data-tooltip="Pinned"
                onclick=self.link.callback(move |e| Msg::Unpin(url_pin.clone()))
                >
                <i class="material-icons">{"star"}</i>
            </a>
            }
        } else {
            html! {
                <a href="#!" class="secondary-content tooltipped search-pinned"
                    data-position="bottom"
                    data-tooltip="Pinned"
                    data-url=url
                    onclick=self.link.callback(move |e| Msg::Pin(url_pin.clone()))
                   >
                    <i class="material-icons">{"star_border"}</i>
                </a>
            }
        }
    }

    fn bookmarked(&self, marked: &i64) -> Html {
        if_html!( marked == &1 =>
         <a href="#!" class="secondary-content tooltipped search-bookmarked" data-position="bottom" data-tooltip="Bookmark">
             <i class="material-icons">{"bookmark"}</i>
         </a>
        )
    }

    fn chip(&self, url: &str, string: &str) -> Html {
        let string = string.trim().to_string();
        let domain = url.to_string().clone();
        if_html!(
            !string.is_empty() =>
                <div class="chip">
                    {string.clone()}
                    <i class="close material-icons" onclick=self.link.callback(move |e: MouseEvent| Msg::Untag((domain.clone(), string.clone())))>{"close"}</i>
                </div>
        )
    }

    fn search_results(&self) -> Html {
        if self.fetching {
            self.loading_html()
        } else if let Some(json) = &self.search_json {
            html! {
            <>
                <ul class="collection">
                    { json.results.iter().map(|i|{ self.search_item_html(&i) }).collect::<Html>() }
                </ul>

                <script>
                    {
                        "
                        var elems = document.querySelectorAll('.dropdown-trigger');
                        var instances = M.Dropdown.init(elems, {closeOnClick: false, constrainWidth: false, container: document.querySelectorAll('.results')});

                        var elems = document.querySelectorAll('.clickclose');
                        for (var i = 0; i < elems.length; i++) {
                            var elem = elems[i];
                            elem.onclick = function () {
                                M.Dropdown._dropdowns.forEach(function (e) {e.close()})
                            };
                        }
                       " 
                    }
                </script>
            </>
            }
        } else {
            html! {
             <ul class="collection">
               <li class="collection-item avatar">
                 <p> { "Search hints:"} <br/>
                 { " music AND (\"wu tang\" OR McConaughey) AND -support - boolean search. quote strings and use parentheses. * wild card, + required, - exclude, ? should" }
                 <br/>

                 { " content:turtle AND url:*wiki* - searchable text fields: content, url, and domain. " }
                 <br/>
                 { " pinned:1 - attributes that are number include hidden, pinned, bookmarked and duplicate. 1 for true and 0 for false. if not used hidden:0 is added to all queries." }
                 <br/>

                 { " tags:/tags/code - tags are hierarchical. the example search will return results for /tags/code/rust. /tags/ are user tags. /keywords/ are the html keywords." }
                 <br/>
                 </p>
               </li>
             </ul>
            }
        }
    }
}

impl App {
    fn setting_modal(&self) -> Html {
        html! {
            <Settings clicked_at=self.settings_click />
        }
    }
    fn content(&self) -> Html {
        html! {
        <div class="row results">
            <div class="col s11">
                <SearchResults search_input=self.search.clone()/>
            </div>
        </div>
        }
    }

    fn header(&self) -> Html {
        html! {
        <header>
            <nav class="top-nav grey darken-3">
                    <div class="nav-wrapper">
                        <a href="#" data-target="slide-out" class="sidenav-trigger brand-logo"><i class="material-icons">{"menu"}</i></a>
                        <div class="input-field">
                            <input id="search" type="search" autocomplete="off" required=true value={self.search.clone()} oninput=self.link.callback(|e: InputData| Msg::Search(e.value))/>
                            <label class="label-icon" for="search"><i class="material-icons">{"search"}</i></label>
                        </div>
                         <a class="btn-floating btn-large halfway-fab waves-effect waves-light grey modal-trigger" href="#setting_modal" onclick=self.link.callback(|_| Msg::ClickSettings)>
                            <i class="material-icons">{"settings"}</i>
                          </a>
                    </div>
            </nav>
        </header>
        }
    }
}

pub enum Msg {
    Search(String),
    Pin(String),
    Unpin(String),
    Hide(String),
    Tag((String, String)),
    Untag((String, String)),
    HideDomain(String),

    //settings
    RemoveIgnoreDomains(String),
    UpdateIgnoreDomains(String),
    IgnoreStrings(String),
    UpdatePort(String),
    ToggleIndexer,

    ClickSettings,
    FetchReady((String, Result<Value, Error>)),
    ViewString(String),
    Ignore,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let _empty: Vec<serde_json::Result<Request<Vec<u8>>>> = vec![];
        let mut param_search = "".to_string();
        let mut show_hash = "".to_string();
        if let Some(location) = document().location() {
            if let Ok(params) = location.search() {
                if params.starts_with("?q=") {
                    ConsoleService::log(&format!("{:?}", params));
                    param_search = params.replace("?q=", "");
                } else if params.starts_with("?view=") {
                    show_hash = params.replace("?view=", "");
                }
            }
        }
        App {
            link,
            search: param_search,
            show_hash: show_hash,
            settings_click: 0,
            // write /read from local stoage
            // https://dev.to/davidedelpapa/yew-tutorial-04-and-services-for-all-1non
            port: "7172".to_string(),
            fetching: false,
            network_task: None,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::ClickSettings => {
                self.settings_click += 1;
            }
            Msg::Search(search_string) => {
                self.search = search_string;
            }
            Msg::FetchReady(_response) => {
                self.fetching = false;
                self.network_task = None;
            }
            Msg::Ignore => {
                return false;
            }
            _ => {}
        }
        true
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        if self.show_hash.is_empty() {
            html! {
            <>
                { self.header() }
                <main>
                    { self.content() }
                    { self.setting_modal() }
                </main>
            </>
            }
        } else {
            html! { <ViewPage hash=self.show_hash.clone() port=self.port.clone()/> }
        }
    }
}

pub struct ViewPage {
    link: ComponentLink<Self>,
    hash: String,
    content: String,
    fetching: bool,
    port: String,
    network_task: Option<yew::services::fetch::FetchTask>,
}

#[derive(Properties, Clone, PartialEq, Debug)]
pub struct ViewPageProps {
    pub hash: String,
    pub port: String,
}

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct ViewJson {
    content: String,
}

impl Component for ViewPage {
    type Message = Msg;
    type Properties = ViewPageProps;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut view = ViewPage {
            hash: props.hash,
            content: "".to_string(),
            fetching: false,
            network_task: None,
            port: props.port,
            link,
        };

        view.fetch_settings(Some(view.port.clone()));
        view
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::ViewString(response) => {
                self.fetching = false;
                self.network_task = None;
                ConsoleService::log(&format!("{:?}", response));
                self.content = response;
            }
            _ => {}
        }
        true
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        if self.fetching {
            html! {
            <div class="progress">
                <div class="indeterminate"></div>
            </div>
                }
        } else if !self.content.is_empty() {
            html! {
                <div class="container">
                    <RawHTML inner_html=self.content.clone()/>
                </div>
            }
        } else {
            html! {}
        }
    }
}

impl ViewPage {
    fn fetch_json(
        &mut self,
        binary: bool,
        url: String,
        stored_data: String,
    ) -> yew::services::fetch::FetchTask {
        let callback = self
            .link
            .callback(move |response: Response<Result<String, Error>>| {
                let (meta, data) = response.into_parts();
                if meta.status.is_success() {
                    Msg::ViewString(data.unwrap_or_else(|_| String::new()))
                } else {
                    Msg::ViewString(String::new())
                }
            });
        let request = Request::get(url)
            .header("Accept", "application/json")
            .body(Nothing)
            .unwrap();
        FetchService::fetch(request, callback).unwrap()
    }
    fn fetch_settings(&mut self, port: Option<String>) {
        self.fetching = true;
        self.network_task = Some(self.fetch_json(
            false,
            format!(
                "http://localhost:{}/view/{}",
                port.unwrap_or_else(|| self.port.clone()),
                self.hash
            ),
            "view".to_string(),
        ));
    }
}
// https://github.com/yewstack/yew/issues/1281
#[derive(Debug, Clone, Eq, PartialEq, Properties)]
struct RawHTMLProps {
    pub inner_html: String,
}

struct RawHTML {
    props: RawHTMLProps,
}

impl Component for RawHTML {
    type Message = Msg;
    type Properties = RawHTMLProps;

    fn create(props: Self::Properties, _: ComponentLink<Self>) -> Self {
        Self { props }
    }

    fn update(&mut self, _: Self::Message) -> ShouldRender {
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.props != props {
            self.props = props;
            true
        } else {
            false
        }
    }

    fn view(&self) -> Html {
        let div = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .create_element("div")
            .unwrap();
        div.set_inner_html(&self.props.inner_html[..]);

        let node = web_sys::Node::from(div);
        let vnode = yew::virtual_dom::VNode::VRef(node);
        vnode
    }
}

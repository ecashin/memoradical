use anyhow::{Context, Result};
use gloo_file::{
    callbacks::{read_as_text, FileReader},
    File,
};
use gloo_storage::{LocalStorage, Storage};
use rand::distributions::WeightedIndex;
use rand_distr::{Beta, Distribution};
use serde::{Deserialize, Serialize};
use web_sys::{Event, HtmlElement, HtmlInputElement};
use yew::prelude::*;

const STORAGE_KEY_CARDS: &str = "net.noserose.memoradical:cards";

enum Msg {
    Flip,
    Help(bool),
    Hit,
    Miss,
    Next,
    Prev,
    StoreCards,
    StoreNewCards(String),
    UploadCards(Vec<File>),
}

#[derive(Debug, Serialize, Deserialize)]
struct Card {
    prompt: String,
    response: String,
    hits: usize,
    misses: usize,
}

#[derive(Debug, PartialEq)]
enum Face {
    Prompt,
    Response,
}

struct Model {
    cards: Vec<Card>,
    current_card: usize,
    display_history: Vec<usize>,
    node_ref: NodeRef,
    readers: Vec<FileReader>,
    showing_help: bool,
    visible_face: Face,
}

fn choose_card(cards: &[Card]) -> usize {
    let rng = &mut rand::thread_rng();
    let weights: Vec<_> = cards
        .iter()
        .map(|card| {
            Beta::new((card.misses + 1) as f64, (card.hits + 1) as f64)
                .unwrap()
                .sample(rng)
        })
        .collect();
    let dist = WeightedIndex::new(&weights).unwrap();
    dist.sample(rng)
}

fn store_data() -> Result<String> {
    let cards: Vec<Card> = vec![
        Card {
            prompt: "What is to the left of right?".to_owned(),
            response: "Left".to_owned(),
            hits: 0,
            misses: 0,
        },
        Card {
            prompt: "What is to the right of left?".to_owned(),
            response: "Right".to_owned(),
            hits: 0,
            misses: 0,
        },
    ];
    let value = serde_json::to_string(&cards).context("serializing cards")?;
    LocalStorage::set(STORAGE_KEY_CARDS, value.clone()).context("storing cards")?;
    Ok(value)
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &yew::Context<Self>) -> Self {
        let retrieved = LocalStorage::get(STORAGE_KEY_CARDS);
        let json: Option<String> = match retrieved {
            Ok(json) => Some(json),
            Err(_e) => match store_data() {
                Ok(json) => Some(json),
                Err(_e) => None,
            },
        };
        let cards: Vec<Card> = serde_json::from_str(&json.unwrap()).unwrap();

        Self {
            cards,
            current_card: 0,
            display_history: vec![],
            visible_face: Face::Prompt,
            readers: vec![],
            node_ref: NodeRef::default(),
            showing_help: true,
        }
    }

    fn rendered(&mut self, _ctx: &yew::Context<Self>, first_render: bool) {
        if first_render {
            if let Some(elt) = self.node_ref.cast::<HtmlElement>() {
                elt.focus().expect("focus on div");
            }
        }
    }

    fn update(&mut self, ctx: &yew::Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Flip => {
                self.visible_face = match self.visible_face {
                    Face::Prompt => Face::Response,
                    Face::Response => Face::Prompt,
                };
                true
            }
            Msg::Help(yesno) => {
                self.showing_help = yesno;
                true
            }
            Msg::Hit => {
                self.cards[self.current_card].hits += 1;
                self.visible_face = Face::Prompt;
                ctx.link().send_message(Msg::StoreCards);
                ctx.link().send_message(Msg::Next);
                true
            }
            Msg::Miss => {
                self.cards[self.current_card].misses += 1;
                self.visible_face = Face::Prompt;
                ctx.link().send_message(Msg::StoreCards);
                ctx.link().send_message(Msg::Next);
                true
            }
            Msg::Next => {
                self.display_history.push(self.current_card);
                self.current_card = choose_card(&self.cards);
                self.visible_face = Face::Prompt;
                true
            }
            Msg::Prev => {
                if !self.display_history.is_empty() {
                    self.current_card = self.display_history.pop().unwrap();
                    true
                } else {
                    false
                }
            }
            Msg::StoreCards => {
                let json = serde_json::to_string(&self.cards).unwrap();
                LocalStorage::set(STORAGE_KEY_CARDS, &json)
                    .context("storing existing cards")
                    .unwrap();
                true
            }
            Msg::StoreNewCards(json) => {
                let cards: Vec<Card> = serde_json::from_str(&json).unwrap();
                self.cards = cards;
                LocalStorage::set(STORAGE_KEY_CARDS, json)
                    .context("storing cards")
                    .unwrap();
                true
            }
            Msg::UploadCards(files) => {
                assert_eq!(files.len(), 1);
                let task = {
                    let link = ctx.link().clone();
                    read_as_text(&files[0], move |result| {
                        link.send_message(Msg::StoreNewCards(result.unwrap()));
                    })
                };
                self.readers.push(task);
                true
            }
        }
    }

    fn view(&self, ctx: &yew::Context<Self>) -> Html {
        let card_html = if let Some(card) = self.cards.get(self.current_card) {
            let text = match self.visible_face {
                Face::Prompt => card.prompt.clone(),
                Face::Response => card.response.clone(),
            };
            html! {
                <>
                    <p>{text}</p>
                </>
            }
        } else {
            html! {}
        };
        let upload_html = html! {
            <div>
                <input type="file" multiple=false
                    onchange={ctx.link().callback(move |e: Event| {
                        let mut result = Vec::new();
                        let input: HtmlInputElement = e.target_unchecked_into();
                        if let Some(files) = input.files() {
                            let files = js_sys::try_iter(&files)
                                .unwrap()
                                .unwrap()
                                .map(|v| web_sys::File::from(v.unwrap()))
                                .map(File::from);
                            result.extend(files);
                        }
                        Msg::UploadCards(result)
                    })}/>
            </div>
        };
        let json_html = {
            let json = serde_json::to_string_pretty(&self.cards).unwrap();
            html! {
                <pre>
                    {json}
                </pre>
            }
        };
        let onkeypress = {
            let link = ctx.link().clone();
            link.batch_callback(|e: yew::events::KeyboardEvent| {
                let k = e.key();
                if k == "f" {
                    Some(Msg::Flip)
                } else if k == "h" {
                    Some(Msg::Hit)
                } else if k == "m" {
                    Some(Msg::Miss)
                } else if k == "n" {
                    Some(Msg::Next)
                } else if k == "p" {
                    Some(Msg::Prev)
                } else {
                    None
                }
            })
        };
        if self.showing_help {
            html! {
                <div>
                    <button onclick={ctx.link().callback(|_| Msg::Help(false))}>{"Go"}</button>
                    <h2>{"Memoradical"}</h2>
                    <p>{"Here is some help."}</p>
                    <hr/>
                    <h2>{"Local Only App"}</h2>
                    <p>{"This web app runs on your browser and stores information on your local system."}</p>
                    <p>
                        <span>{"Your information never leaves your system."}</span>
                        <span>{"It only requests HTML and "}</span>
                        <a href="https://webassembly.org/">{"Web Assembly"}</a>
                        <span>{" from the server."}</span>
                    </p>
                    <hr/>
                    <h2>{"Usage"}</h2>
                    <p>{"To flip the card, click \"Flip\" or hit the \"f\" key."}</p>
                    <p>{"If you know the meaning of the word, click \"Hit\" or hit the \"h\" key."}</p>
                    <p>{"If you know the meaning of the word, click \"Miss\" or hit the \"m\" key."}</p>
                    <p>{"To go to the next card without hitting or missing, click \"Next\" or hit the \"n\" key."}</p>
                    <p>
                        <span>{"To go to the previous card without hitting or missing, click \"Prev\" or hit the \"p\" key."}</span>
                        <span>{" After you go backward, going forward results in new random draws for cards."}</span>
                    </p>
                    <hr/>
                    <h2>{"Data"}</h2>
                    <p>{"Use the button at the top to upload a JSON file with new cards."}</p>
                    <p>{"Follow the format of the JSON displayed at the bottom of the page."}</p>
                    <p>{"Misses make cards appear more frequently, but hits make them appear less frequently."}</p>
                    <p>{"If no data is available, two dummy cards are displayed."}</p>
                </div>
            }
        } else {
            html! {
                <div id="memoradical" {onkeypress}>
                    <button onclick={ctx.link().callback(|_| Msg::Help(true))}>{"Help"}</button>
                    {upload_html}
                    {card_html}
                    <button ref={self.node_ref.clone()}
                        onclick={ctx.link().callback(|_| Msg::Flip)}>{ "Flip" }</button>
                    <button onclick={ctx.link().callback(|_| Msg::Prev)}>{ "Prev" }</button>
                    <button onclick={ctx.link().callback(|_| Msg::Next)}>{ "Next" }</button>
                    <button onclick={ctx.link().callback(|_| Msg::Hit)}>{ "Hit" }</button>
                    <button onclick={ctx.link().callback(|_| Msg::Miss)}>{ "Miss" }</button>
                    {json_html}
                </div>
            }
        }
    }
}

fn main() {
    yew::start_app::<Model>();
}

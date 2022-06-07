use anyhow::{Context, Result};
use gloo_file::{
    callbacks::{read_as_text, FileReader},
    File,
};
use gloo_storage::{LocalStorage, Storage};
use rand::distributions::WeightedIndex;
use rand_distr::{Beta, Distribution};
use serde::{Deserialize, Serialize};
use web_sys::{Event, HtmlInputElement};
use yew::prelude::*;

const STORAGE_KEY_CARDS: &str = "net.noserose.memoradical:cards";

enum Msg {
    Hit,
    Miss,
    Next,
    StoreCards(String),
    UploadCards(Vec<File>),
}

#[derive(Debug, Serialize, Deserialize)]
struct Card {
    prompt: String,
    response: String,
    hits: usize,
    misses: usize,
}

struct Model {
    cards: Vec<Card>,
    current_card: usize,
    readers: Vec<FileReader>,
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
            Err(_) => match store_data() {
                Ok(json) => Some(json),
                Err(_) => None,
            },
        };
        let cards: Vec<Card> = serde_json::from_str(&json.unwrap()).unwrap();

        Self {
            cards,
            current_card: 0,
            readers: vec![],
        }
    }

    fn update(&mut self, ctx: &yew::Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Hit => {
                self.cards[self.current_card].hits += 1;
                ctx.link().send_message(Msg::Next);
                true
            }
            Msg::Miss => {
                self.cards[self.current_card].misses += 1;
                ctx.link().send_message(Msg::Next);
                true
            }
            Msg::Next => {
                self.current_card = choose_card(&self.cards);
                true
            }
            Msg::StoreCards(json) => {
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
                        link.send_message(Msg::StoreCards(result.unwrap()));
                    })
                };
                self.readers.push(task);
                true
            }
        }
    }

    fn view(&self, ctx: &yew::Context<Self>) -> Html {
        let card_html = if let Some(card) = self.cards.get(self.current_card) {
            html! {
                <>
                    <p>{format!("{:?}", card)}</p>
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
        html! {
            <div>
                {upload_html}
                {card_html}
                <button onclick={ctx.link().callback(|_| Msg::Next)}>{ "Next" }</button>
                <button onclick={ctx.link().callback(|_| Msg::Hit)}>{ "Hit" }</button>
                <button onclick={ctx.link().callback(|_| Msg::Miss)}>{ "Miss" }</button>
                {json_html}
            </div>
        }
    }
}

fn main() {
    yew::start_app::<Model>();
}

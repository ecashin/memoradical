use std::cmp::Ordering;
use std::collections::{HashSet, LinkedList};

use anyhow::{anyhow, Context, Result};
use gloo_console::console_dbg;
use gloo_file::{
    callbacks::{read_as_text, FileReader},
    File,
};
use gloo_storage::{LocalStorage, Storage};
use gloo_timers::callback::Interval;
use rand::distributions::WeightedIndex;
use rand_distr::{Beta, Distribution};
use serde::{Deserialize, Serialize};
use web_sys::{Event, HtmlElement, HtmlInputElement};
use yew::prelude::*;

const COPY_BORDER_FADE_MS: u32 = 50;
const STORAGE_KEY_CARDS: &str = "net.noserose.memoradical:cards";

enum Msg {
    AddCard,
    AddMode,
    AllCardsMode,
    CopyCards,
    CopyCardsSuccess,
    DeleteCard(usize),
    Edit(Option<usize>), // None means self's current card
    FadeCopyBorder,
    Flip,
    HelpMode,
    Hit,
    Miss,
    Next,
    Noop,
    Prev,
    ReverseModeToggle,
    StatsMode,
    StoreCards,
    StoreNewCards(String),
    StudyMode,
    UpdateNewBackText(String),
    UpdateNewFrontText(String),
    UploadCards(Vec<File>),
}

#[derive(PartialEq)]
enum Mode {
    Add,
    AllCards,
    Edit,
    Help,
    Stats,
    Study,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Card {
    prompt: String,
    response: String,
    hits: usize,
    misses: usize,
    reverse_hits: Option<usize>,
    reverse_misses: Option<usize>,
}

impl Card {
    fn new(front: &str, back: &str) -> Card {
        Card {
            prompt: front.to_string(),
            response: back.to_string(),
            hits: 0,
            misses: 0,
            reverse_hits: None,
            reverse_misses: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Face {
    Prompt,
    Response,
}

impl Face {
    fn other_side(one: &Face) -> Face {
        match one {
            Face::Prompt => Face::Response,
            Face::Response => Face::Prompt,
        }
    }
}

struct Model {
    cards: Vec<Card>,
    copy_border_opacity: f32,
    copy_border_fader: Option<Interval>,
    current_card: Option<usize>,
    display_history: LinkedList<usize>,
    new_front_text: String,
    new_back_text: String,
    node_ref: NodeRef,
    readers: Vec<FileReader>,
    mode: Mode,
    need_key_focus: bool,
    visible_face: Face,
    reverse_mode: bool,
    deletion_target: Option<usize>,
}

fn store_data() -> Result<String> {
    let reverse_hits = None;
    let reverse_misses = None;
    let cards: Vec<Card> = vec![
        Card {
            prompt: "What is the key for flipping a card?".to_owned(),
            response: "\"f\"".to_owned(),
            hits: 0,
            misses: 0,
            reverse_hits,
            reverse_misses,
        },
        Card {
            prompt: "What is the key for registering a \"hit\"?".to_owned(),
            response: "\"h\"".to_owned(),
            hits: 0,
            misses: 0,
            reverse_hits,
            reverse_misses,
        },
        Card {
            prompt: "What is the key for registering a \"miss\"?".to_owned(),
            response: "\"m\"".to_owned(),
            hits: 0,
            misses: 0,
            reverse_hits,
            reverse_misses,
        },
        Card {
            prompt: "What key shows the previous card?".to_owned(),
            response: "\"p\"".to_owned(),
            hits: 0,
            misses: 0,
            reverse_hits,
            reverse_misses,
        },
        Card {
            prompt: "What key shows the next card without registering hit or miss?".to_owned(),
            response: "\"n\"".to_owned(),
            hits: 0,
            misses: 0,
            reverse_hits,
            reverse_misses,
        },
        Card {
            prompt: "What is the key for editing the current card?".to_owned(),
            response: "\"e\"".to_owned(),
            hits: 0,
            misses: 0,
            reverse_hits,
            reverse_misses,
        },
    ];
    let value = serde_json::to_string(&cards).context("serializing cards")?;
    LocalStorage::set(STORAGE_KEY_CARDS, value.clone()).context("storing cards")?;
    Ok(value)
}

impl Model {
    fn change_mode(&mut self, new_mode: Mode) {
        if new_mode == Mode::Study && self.mode != Mode::Study {
            self.need_key_focus = true;
        }
        self.mode = new_mode;
    }

    fn choose_card(&self) -> usize {
        let rng = &mut rand::thread_rng();
        let history: HashSet<_> = self.display_history.iter().copied().collect();
        let weights: Vec<_> = self
            .cards
            .iter()
            .enumerate()
            .map(|(i, card)| {
                if history.contains(&i) {
                    0.0
                } else {
                    let misses = if self.reverse_mode {
                        card.reverse_misses.unwrap_or_default()
                    } else {
                        card.misses
                    };
                    let hits = if self.reverse_mode {
                        card.reverse_hits.unwrap_or_default()
                    } else {
                        card.hits
                    };
                    let shape1 = misses + 1;
                    let shape2 = hits + 1;
                    Beta::new(shape1 as f64, shape2 as f64).unwrap().sample(rng)
                }
            })
            .collect();
        let dist = WeightedIndex::new(&weights).unwrap();
        dist.sample(rng)
    }

    fn copy_button_style(&self) -> String {
        if self.copy_border_opacity == 0.0 {
            return "".to_owned();
        }
        format!(
            "border-radius: 15%; border-width: thick; border-color: rgba(10, 220, 10, {})",
            self.copy_border_opacity
        )
    }

    fn pop_last_displayed(&mut self) -> Option<usize> {
        self.display_history.pop_back()
    }

    fn record_display(&mut self, card: usize) {
        let n = (self.cards.len() as f64).log2().round() as usize;
        self.display_history.push_back(card);
        if self.display_history.len() > n {
            self.display_history.pop_front();
        }
    }

    fn stats_html(&self) -> Html {
        let mut cards = self.cards.clone();
        let r = |card: &Card| {
            let total = card.hits + card.misses;
            if total == 0 {
                0.0
            } else {
                card.hits as f32 / total as f32
            }
        };
        cards.sort_by(|a, b| r(b).partial_cmp(&r(a)).unwrap());
        let rows = cards
            .iter()
            .map(|c| {
                let percent = r(c) * 100.0;
                let rhits = c.reverse_hits.unwrap_or_default();
                let rmisses = c.reverse_misses.unwrap_or_default();
                let rpercent = {
                    let total = rhits + rmisses;
                    if total == 0 {
                        0.0
                    } else {
                        (rhits as f32 / total as f32) * 100.0
                    }
                };
                html! {
                    <tr>
                        <td>{&c.prompt}</td>
                        <td>{&c.response}</td>
                        <td class="number">{c.hits}</td>
                        <td class="number">{c.misses}</td>
                        <td class="number">{format!("{:.2}", percent)}</td>
                        <td class="number">{rhits}</td>
                        <td class="number">{rmisses}</td>
                        <td class="number">{format!("{:.2}", rpercent)}</td>
                    </tr>
                }
            })
            .collect::<Vec<_>>();
        html! {
            <table class="striped">
                <tr>
                    <th>{"prompt"}</th>
                    <th>{"response"}</th>
                    <th>{"hits"}</th>
                    <th>{"misses"}</th>
                    <th>{"percent hit"}</th>
                    <th>{"reverse hits"}</th>
                    <th>{"reverse misses"}</th>
                    <th>{"reverse percent hit"}</th>
                </tr>
                {rows}
            </table>
        }
    }
}

async fn copy_cards_to_clipboard(cards: &[Card]) -> Result<()> {
    let value = serde_json::to_string_pretty(cards).context("serializing cards")?;
    let navigator: web_sys::Navigator = web_sys::window().unwrap().navigator();
    console_dbg!("clipboard write");
    if let Some(clipboard) = navigator.clipboard() {
        let write_promise = clipboard.write_text(&value);
        let result = wasm_bindgen_futures::JsFuture::from(write_promise).await;
        if let Err(e) = result {
            Err(anyhow!("Cannot copy to clipboard: {:?}", e))
        } else {
            Ok(())
        }
    } else {
        Err(anyhow!("Cannot obtain clipboard from browser"))
    }
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
        let current_card = None;
        Self {
            cards,
            copy_border_opacity: 0.0,
            copy_border_fader: None,
            current_card,
            display_history: LinkedList::new(),
            new_back_text: "".to_owned(),
            new_front_text: "".to_owned(),
            visible_face: Face::Prompt,
            readers: vec![],
            node_ref: NodeRef::default(),
            mode: Mode::Help,
            need_key_focus: true,
            reverse_mode: false,
            deletion_target: None,
        }
    }

    fn rendered(&mut self, _ctx: &yew::Context<Self>, _first_render: bool) {
        if self.need_key_focus {
            if let Some(elt) = self.node_ref.cast::<HtmlElement>() {
                elt.focus().expect("focus on div");
            }
        }
    }

    fn update(&mut self, ctx: &yew::Context<Self>, msg: Self::Message) -> bool {
        let need_render = match msg {
            Msg::AddCard => {
                if self.mode == Mode::Edit {
                    self.cards[self.current_card.unwrap()].prompt = self.new_front_text.clone();
                    self.cards[self.current_card.unwrap()].response = self.new_back_text.clone();
                    self.change_mode(Mode::Study);
                } else {
                    let card = Card::new(&self.new_front_text, &self.new_back_text);
                    self.cards.push(card);
                }
                self.new_back_text = "".to_owned();
                self.new_front_text = "".to_owned();
                ctx.link().send_message(Msg::StoreCards);
                true
            }
            Msg::AddMode => {
                self.change_mode(Mode::Add);
                true
            }
            Msg::AllCardsMode => {
                self.change_mode(Mode::AllCards);
                true
            }
            Msg::CopyCards => {
                let cards = self.cards.clone();
                ctx.link().send_future(async move {
                    match copy_cards_to_clipboard(&cards).await {
                        Err(e) => {
                            console_dbg!(&e);
                            Msg::Noop
                        }
                        Ok(_) => Msg::CopyCardsSuccess,
                    }
                });
                true
            }
            Msg::CopyCardsSuccess => {
                self.copy_border_opacity = 1.0;
                let handle = {
                    let link = ctx.link().clone();
                    Interval::new(COPY_BORDER_FADE_MS, move || {
                        link.send_message(Msg::FadeCopyBorder)
                    })
                };
                self.copy_border_fader = Some(handle);
                true
            }
            Msg::DeleteCard(i) => {
                if self.deletion_target.is_some() && self.deletion_target.unwrap() == i {
                    if let Some(curr) = self.current_card {
                        self.current_card = match curr.cmp(&i) {
                            Ordering::Equal => None,
                            Ordering::Greater => Some(curr - 1),
                            Ordering::Less => Some(curr),
                        };
                    }
                    self.display_history.clear(); // ... because the numbers changed
                    self.cards.remove(i);
                    ctx.link().send_message(Msg::StoreCards);
                    self.deletion_target = None;
                } else {
                    self.deletion_target = Some(i);
                }
                true
            }
            Msg::Edit(i) => {
                let mut redraw = false;
                let card_index = if i.is_none() { self.current_card } else { i };
                if let Some(i) = card_index {
                    if let Some(card) = self.cards.get(i) {
                        redraw = true;
                        self.current_card = Some(i);
                        self.new_front_text = card.prompt.clone();
                        self.new_back_text = card.response.clone();
                        self.change_mode(Mode::Edit);
                    }
                }
                redraw
            }
            Msg::FadeCopyBorder => {
                self.copy_border_opacity *= 0.9;
                if self.copy_border_opacity < 0.2 {
                    self.copy_border_fader = None;
                    self.copy_border_opacity = 0.0;
                }
                true
            }
            Msg::Flip => {
                self.visible_face = match self.visible_face {
                    Face::Prompt => Face::Response,
                    Face::Response => Face::Prompt,
                };
                true
            }
            Msg::HelpMode => {
                self.change_mode(Mode::Help);
                true
            }
            Msg::Hit => {
                if let Some(card) = self.current_card {
                    if self.reverse_mode {
                        let rhits = self.cards[card].reverse_hits;
                        self.cards[card].reverse_hits = Some(rhits.map_or(1, |v| v + 1));
                    } else {
                        self.cards[card].hits += 1;
                    }
                    self.visible_face = Face::Prompt;
                    ctx.link().send_message(Msg::StoreCards);
                    ctx.link().send_message(Msg::Next);
                    true
                } else {
                    false
                }
            }
            Msg::Miss => {
                if let Some(card) = self.current_card {
                    if self.reverse_mode {
                        let rmisses = self.cards[card].reverse_misses;
                        self.cards[card].reverse_misses = Some(rmisses.map_or(1, |v| v + 1));
                    } else {
                        self.cards[card].misses += 1;
                    }
                    self.visible_face = Face::Prompt;
                    ctx.link().send_message(Msg::StoreCards);
                    ctx.link().send_message(Msg::Next);
                    true
                } else {
                    false
                }
            }
            Msg::Next => {
                if self.current_card.is_some() {
                    self.record_display(self.current_card.unwrap());
                }
                self.current_card = Some(self.choose_card());
                self.visible_face = Face::Prompt;
                true
            }
            Msg::Noop => false,
            Msg::Prev => {
                if let Some(last_card) = self.pop_last_displayed() {
                    self.current_card = Some(last_card);
                    self.visible_face = Face::Prompt;
                    true
                } else {
                    false
                }
            }
            Msg::ReverseModeToggle => {
                self.reverse_mode = !self.reverse_mode;
                true
            }
            Msg::StatsMode => {
                self.change_mode(Mode::Stats);
                true
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
                self.current_card = Some(self.choose_card());
                self.visible_face = Face::Prompt;
                LocalStorage::set(STORAGE_KEY_CARDS, json)
                    .context("storing cards")
                    .unwrap();
                true
            }
            Msg::StudyMode => {
                self.change_mode(Mode::Study);
                true
            }
            Msg::UpdateNewBackText(text) => {
                self.new_back_text = text;
                true
            }
            Msg::UpdateNewFrontText(text) => {
                self.new_front_text = text;
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
        };
        if self.mode == Mode::Study && self.current_card.is_none() {
            self.current_card = Some(self.choose_card());
            true
        } else {
            need_render
        }
    }

    fn view(&self, ctx: &yew::Context<Self>) -> Html {
        let mode_buttons = html! {
            <div>
                <button disabled={self.mode == Mode::Help} onclick={ctx.link().callback(|_| Msg::HelpMode)}>{"Help"}</button>
                <button disabled={self.mode == Mode::Study} onclick={ctx.link().callback(|_| Msg::StudyMode)}>{"Study"}</button>
                <button disabled={self.mode == Mode::Add || self.mode == Mode::Edit} onclick={ctx.link().callback(|_| Msg::AddMode)}>{"Add Card"}</button>
                <button disabled={self.mode == Mode::AllCards} onclick={ctx.link().callback(|_| Msg::AllCardsMode)}>{"All Cards"}</button>
                <button disabled={self.mode == Mode::Stats} onclick={ctx.link().callback(|_| Msg::StatsMode)}>{"Stats"}</button>
            </div>
        };
        let add_card_html = html! {
            <div>
                <input
                id="new-front" type="text" value={self.new_front_text.clone()}
                oninput={ctx.link().callback(|e: InputEvent| {
                        let input = e.target_unchecked_into::<HtmlInputElement>();
                        Msg::UpdateNewFrontText(input.value())
                })}
                />
                <input
                id="new-back" type="text" value={self.new_back_text.clone()}
                oninput={ctx.link().callback(|e: InputEvent| {
                        let input = e.target_unchecked_into::<HtmlInputElement>();
                        Msg::UpdateNewBackText(input.value())
                })}
                />
                <button
                    onclick={ctx.link().callback(|_| Msg::AddCard)}
                >{
                    if self.mode == Mode::Edit {
                        "Update Card"
                    } else {
                        "Add Card"
                    }
                }</button>
            </div>
        };
        let card_html = if let Some(card_index) = self.current_card {
            let card = &self.cards[card_index];
            let face = if self.reverse_mode {
                Face::other_side(&self.visible_face)
            } else {
                self.visible_face.clone()
            };
            let (text, bg_color) = match face {
                Face::Prompt => (card.prompt.clone(), "#EEE8AA"),
                Face::Response => (card.response.clone(), "#C1FFC1"),
            };
            let style = format!("background-color: {bg_color}; font-size: large; padding: 3em");
            html! {
                <>
                    <p style={style}>{text}</p>
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
                <button
                    onclick={ctx.link().callback(move |_| Msg::CopyCards)}
                    style={self.copy_button_style()}
                >
                    {"Copy Cards to Clipboard"}
                </button>
            </div>
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
                } else if k == "e" {
                    Some(Msg::Edit(None))
                } else {
                    None
                }
            })
        };
        let reverse_mode_html = html! {
            <div class="form-check">
                <input
                    id="reverse-mode-checkbox"
                    class="form-check-input"
                    type={"checkbox"}
                    value=""
                    checked={ self.reverse_mode }
                    autocomplete={"off"}
                    onclick={ctx.link().callback(move |_| Msg::ReverseModeToggle)}
                />
                <label
                    class="form-check-label"
                    for="reverse-mode-checkbox">{"reverse mode"}
                </label>
            </div>
        };
        match self.mode {
            Mode::AllCards => {
                let mut cards_html = vec![];
                for (i, card) in self.cards.iter().enumerate() {
                    let delete_button_label =
                        if self.deletion_target.is_some() && self.deletion_target.unwrap() == i {
                            "Really? DELETE!"
                        } else {
                            "Delete"
                        };
                    let edit_button = html! {
                        <button onclick={ctx.link().callback(move |_| Msg::Edit(Some(i)))}>
                            {"Edit"}
                        </button>
                    };
                    let delete_button = html! {
                        <button onclick={ctx.link().callback(move |_| Msg::DeleteCard(i))}>
                            {delete_button_label}
                        </button>
                    };
                    cards_html.push(html! {
                        <tr>
                            <td>{&card.prompt}</td>
                            <td>{&card.response}</td>
                            <td>{edit_button}</td>
                            <td>{delete_button}</td>
                        </tr>
                    });
                }
                html! {
                    <div>
                        {mode_buttons}
                        {upload_html}
                        <table class="striped">
                            <tr>
                                <th>{"Prompt"}</th>
                                <th>{"Response"}</th>
                            </tr>
                            {cards_html}
                        </table>
                    </div>
                }
            }
            Mode::Help => {
                let title = format!("Memoradical v{}", env!("CARGO_PKG_VERSION"));
                html! {
                    <div>
                        {mode_buttons}
                        <h2>{title}</h2>
                        <p>{"Here is some help for "}<a href="https://github.com/ecashin/memoradical">{"Memoradical"}</a>{"."}</p>
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
                        <p>{"To edit a card, click the \"Edit\" button or hit \"e\"."}</p>
                        <p>{"(You might need to click a button before using the keyboard shortcuts.)"}</p>
                        <p>{"Check the \"reverse mode\" checkbox to use the other side of the cards as prompts."}</p>
                        <p>{"Adding cards is supported, but copy, edit, and upload JSON for other card modifications."}</p>
                        <hr/>
                        <h2>{"Data"}</h2>
                        <p>{"Use the button at the top to upload a JSON file with new cards."}</p>
                        <p>{"Follow the format of the JSON displayed at the bottom of the page."}</p>
                        <p>{"Misses make cards appear more frequently, but hits make them appear less frequently."}</p>
                        <p>{"If no data is available, two dummy cards are displayed."}</p>
                        <h2>{"Tips"}</h2>
                        <p>{"After going through a few cards, use \"p\" to go back through recent history."}</p>
                        <p>{"If you still don't remember, you can record another miss and use \"p\" again twice to resume time travel."}</p>
                        <p>{"The history is limited to a length on the order of the logarithm of the number of cards."}</p>
                    </div>
                }
            }
            Mode::Stats => {
                html! {
                    <div>
                        {mode_buttons}
                        {self.stats_html()}
                    </div>
                }
            }
            Mode::Study => {
                html! {
                    <div id="memoradical" {onkeypress}>
                        {mode_buttons}
                        <br/>
                        {reverse_mode_html}
                        {card_html}
                        <button ref={self.node_ref.clone()}
                            onclick={ctx.link().callback(|_| Msg::Flip)}>{ "Flip" }</button>
                        <button onclick={ctx.link().callback(|_| Msg::Prev)}>{ "Prev" }</button>
                        <button onclick={ctx.link().callback(|_| Msg::Next)}>{ "Next" }</button>
                        <button onclick={ctx.link().callback(|_| Msg::Hit)}>{ "Hit" }</button>
                        <button onclick={ctx.link().callback(|_| Msg::Miss)}>{ "Miss" }</button>
                        <button onclick={ctx.link().callback(|_| Msg::Edit(None))}>{ "Edit" }</button>
                    </div>
                }
            }
            Mode::Add | Mode::Edit => {
                html! {
                    <div>
                        {mode_buttons}
                        {add_card_html}
                    </div>
                }
            }
        }
    }
}

fn main() {
    yew::start_app::<Model>();
}

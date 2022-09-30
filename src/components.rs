use std::cell::RefCell;

use yew::prelude::*;

use crate::{Card, GOODNESS_CRITERION};

pub fn mean(x: &[f32]) -> f32 {
    if x.is_empty() {
        0.0
    } else {
        x.iter().sum::<f32>() / x.len() as f32
    }
}

#[derive(Properties, PartialEq)]
pub struct StatsProps {
    pub cards: RefCell<Vec<Card>>,
    pub n_rows_displayed: usize,
    pub reverse_mode: bool,
}

#[function_component(Stats)]
pub fn stats(props: &StatsProps) -> Html {
    let mut cards = props.cards.borrow_mut();

    if cards.is_empty() {
        return html! {
            <p>{"There are no cards."}</p>
        };
    }
    let hits_misses = |card: &Card| {
        if props.reverse_mode {
            (
                card.reverse_hits.unwrap_or_default(),
                card.reverse_misses.unwrap_or_default(),
            )
        } else {
            (card.hits, card.misses)
        }
    };
    let hit_ratio = |h, m| {
        let total = h + m;
        if total == 0 {
            0.0
        } else {
            h as f32 / total as f32
        }
    };
    let r = |card: &Card| {
        let (h, m) = hits_misses(card);
        hit_ratio(h, m)
    };
    let goodness = |card: &Card| {
        let (h, m) = hits_misses(card);
        let total = h + m;
        if total == 0 {
            0.0
        } else {
            let diff = h as isize - m as isize;
            diff as f32 / total as f32
        }
    };
    cards.sort_by(|a, b| goodness(b).partial_cmp(&goodness(a)).unwrap());
    let percent_visited = 100.0
        * (cards
            .iter()
            .filter(|c| {
                let (h, m) = hits_misses(c);
                h + m > 0
            })
            .count() as f32)
        / cards.len() as f32;
    let n_responses = cards
        .iter()
        .map(|c| {
            let (h, m) = hits_misses(c);
            h + m
        })
        .sum::<usize>();
    let percents = cards.iter().map(|c| r(c) * 100.0).collect::<Vec<_>>();
    let goodnesses = cards.iter().map(|c| goodness(c)).collect::<Vec<_>>();
    let rows = cards
        .iter()
        .take(props.n_rows_displayed)
        .zip(percents.iter())
        .zip(goodnesses.iter())
        .map(|((c, percent), good)| {
            let (h, m) = hits_misses(c);
            html! {
                <tr>
                    <td>{&c.prompt}</td>
                    <td>{&c.response}</td>
                    <td class="number">{h}</td>
                    <td class="number">{m}</td>
                    <td class="number">{format!("{:.2}", percent)}</td>
                    <td class="number">{format!("{:.2}", good)}</td>
                </tr>
            }
        })
        .collect::<Vec<_>>();
    let prefix = if props.reverse_mode { "reverse " } else { "" };
    let percent_good = {
        let ratio = if goodnesses.is_empty() {
            0.0
        } else {
            let n_good = goodnesses
                .iter()
                .zip(cards.iter())
                .filter(|(g, c)| {
                    let (h, m) = hits_misses(c);
                    h + m > 1  // just one response isn't enough to "know it well"
                    && *g >= &GOODNESS_CRITERION
                })
                .count();
            n_good as f32 / goodnesses.len() as f32
        };
        100.0 * ratio
    };
    html! {
        <>
            <ul>
                <li>
                    <span class="tooltip">
                        <span class="tooltiptext">
                            {"Average per card"}
                            <br />
                            {"(hits - misses) / (hits + misses)"}
                        </span>
                        {"Overall score: "}{format!("{:.2}", 100.0 * mean(&goodnesses))}
                    </span>
                </li>
                <li>
                    <span class="tooltip">
                        <span class="tooltiptext">
                            {"Visited more than once and with"}
                            <br />
                            {format!("(hits - misses) / (hits + misses) > {:.2}", GOODNESS_CRITERION)}
                        </span>
                        {"Cards known well: "}{format!("{:.2}%", percent_good)}
                    </span>
                </li>
                <li>
                    {"Cards visited: "}
                    {format!("{:.2}% of {}", percent_visited, cards.len())}
                </li>
                <li>{"Number of responses: "}{format!("{n_responses}")}</li>
            </ul>
            <table class="striped">
                <tr>
                    <th>{"prompt"}</th>
                    <th>{"response"}</th>
                    <th>{format!("{}hits", prefix)}</th>
                    <th>{format!("{}misses", prefix)}</th>
                    <th>{format!("{}percent hit", prefix)}</th>
                    <th>{format!("{}goodness", prefix)}</th>
                </tr>
                {rows}
            </table>
        </>
    }
}

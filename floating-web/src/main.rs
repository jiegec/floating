use std::io::Cursor;

use floating_cli::process_arg;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{Event, HtmlInputElement};
use yew::prelude::*;

// https://github.com/yewstack/yew/blob/84b7548bf7b7640c92d2f73282a4df16cde6ca36/examples/password_strength/src/text_input.rs#L11
fn get_value_from_input_event(e: Event) -> String {
    let event_target = e.target().unwrap_throw();
    let target: HtmlInputElement = event_target.dyn_into().unwrap_throw();
    web_sys::console::log_1(&target.value().into());
    target.value()
}

#[function_component]
fn App() -> Html {
    let input = use_state(|| String::new());
    let input_value = (*input).clone();
    let result = use_state(|| String::new());
    let result_value = (*result).clone();

    let oninput = Callback::from(move |input_event: Event| {
        let new_input = get_value_from_input_event(input_event);
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);
        for part in new_input.split(" ") {
            process_arg(&mut cursor, part).unwrap();
        }
        input.set(new_input);
        result.set(String::from_utf8_lossy(&buffer).into_owned());
    });

    html! {
        <div>
            <h1>{"Input:"}</h1>
            <br/>
            <input type="text" value={input_value} onchange={oninput} />
            <br/>
            <h1>{"Result:"}</h1>
            <br/>
            <pre>
                {result_value}
            </pre>
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}

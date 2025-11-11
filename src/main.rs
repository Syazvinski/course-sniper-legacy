use async_std::task::sleep;
use chromiumoxide::error::CdpError;
use chromiumoxide::page::ScreenshotParams;
use chromiumoxide::{Browser, BrowserConfig, Element, Page};
use chrono::{Local, Timelike};
use clap::Parser;
use core::fmt;
use elements::{EmoryPageElements, ToTable};
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::{MultiSelect, Password, PasswordDisplayMode, Select, Text};
use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

mod args;
use args::SniperArgs;

mod ascii;
mod elements;

const TIMEOUT: u64 = 120;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // get args
    let cli_args = SniperArgs::parse();

    println!("\n{}\n", ascii::BANNER);
    println!("Welcome to course-sniper, the precision registration tool.");

    let pb = get_progress_bar("Enabling browser...");

    // setup browser
    let (mut browser, mut handler) = if cli_args.attach {
        Browser::launch(BrowserConfig::builder().with_head().build()?).await?
    } else {
        Browser::launch(BrowserConfig::builder().build()?).await?
    };

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    let handle = async_std::task::spawn(async move {
        while running_clone.load(Ordering::Relaxed) {
            if let Some(event) = handler.next().await {
                let _ = event;
            }
        }
    });

    browser.clear_cookies().await?;
    pb.finish_with_message("Browser enabled.");

    // page elements
    let elements = elements::EmoryPageElements::default();

    let page = browser.new_page(elements.page_url).await?;
    page.enable_stealth_mode().await?;

    match run(&page, elements).await {
        Ok(_) => (),
        Err(e) => {
            if cli_args.debug {
                page.save_screenshot(
                    ScreenshotParams::builder().full_page(true).build(),
                    format!(
                        "debug-{}.png",
                        Local::now().format("%H:%M:%S.%3f").to_string()
                    ),
                )
                .await?;
            }
            Err(e)?
        }
    }

    // cleanup
    browser.close().await?;
    browser.try_wait()?;
    running.store(false, Ordering::Relaxed);
    handle.await;
    Ok(())
}

async fn run(page: &Page, elements: EmoryPageElements) -> Result<(), Box<dyn std::error::Error>> {
    // login info
    let user_name = Text::new("Username: ").prompt()?;
    let user_pwd = Password::new("Password: ")
        .with_display_mode(PasswordDisplayMode::Masked)
        .without_confirmation()
        .prompt()?;

    let pb = get_progress_bar("Logging in with credentials...");

    // login
    page.wait_for_navigation()
        .await?
        .find_element(elements.username_input)
        .await?
        .click()
        .await?
        .type_str(user_name)
        .await?;
    page.find_element(elements.passwd_input)
        .await?
        .click()
        .await?
        .type_str(user_pwd)
        .await?
        .press_key("Enter")
        .await?;

    // authentication transition
    match authentication_transition(&page, &elements, TIMEOUT).await {
        Ok(status) => match status {
            AuthTransition::AuthSuccess => pb.finish_with_message("Authenticated."),
            AuthTransition::AuthFail => {
                pb.finish_with_message("Invalid credentials.");
                return Ok(());
            }
            AuthTransition::Duo => {
                pb.finish_with_message("Duo authentication required.");
                let pb = get_progress_bar("Waiting for Duo confirmation...");
                match duo_transition(&page, &elements, TIMEOUT).await {
                    Ok(status) => match status {
                        DuoTransition::Trust => pb.finish_with_message("Authenticated."),
                        DuoTransition::TimeOut => {
                            pb.finish_with_message("Duo authentication timed out.");
                            return Ok(());
                        }
                        DuoTransition::Cart => pb.finish_with_message("Authenticated."),
                    },
                    Err(e) => {
                        pb.finish_with_message("Failed to find the correct elements or timed out.");
                        Err(e)?
                    }
                }
            }
        },
        Err(e) => {
            pb.finish_with_message("Failed to find the correct elements or timed out.");
            Err(e)?
        }
    }

    // pick a shopping cart
    let pb = get_progress_bar("Looking for shopping cart...");
    match cart_transition(&page, &elements, TIMEOUT).await {
        Ok(status) => match status {
            CartTransition::In => pb.finish_with_message("Entered shopping cart."),
            CartTransition::Select => {
                pb.finish_with_message("Shopping carts found.");
                let carts = elements.get_shopping_carts(&page).await?;
                let selected_cart = Select::new("Select a cart:", carts).prompt()?;
                selected_cart.element.click().await?;
            }
        },
        Err(e) => {
            pb.finish_with_message("Failed to find the correct elements or timed out.");
            Err(e)?
        }
    }

    // get course info
    let pb = get_progress_bar("Fetching courses in cart...");
    wait_element_agressive_retry(&page, elements.course_row, TIMEOUT).await?;
    let courses = elements.get_cart_courses(&page).await?;
    pb.finish_with_message(format!("Found {} courses.", courses.len()));
    println!("{}", courses.to_table());

    // pick courses
    let selected_courses = MultiSelect::new("Select courses:", courses).prompt()?;

    // pick validate or enroll
    if Select::new("Select action:", vec!["Validate", "Enroll"]).prompt()? == "Enroll" {
        //TODO improve registration time selection and implimentation
        let registration_times: Vec<RegistrationTime> = (1..=12)
            .flat_map(|hour| {
                (0..60).flat_map(move |minute| {
                    [true, false]
                        .iter()
                        .map(move |&am| RegistrationTime(hour, minute, am))
                })
            })
            .collect();
        let registration_time =
            Select::new("Select registration time:", registration_times).prompt()?;
        let pb = get_progress_bar(format!(
            "Waiting for registration time: {registration_time}..."
        ));
        let registration_hour = if registration_time.2 {
            registration_time.0
        } else {
            registration_time.0 + 12
        };
        loop {
            let now = Local::now();
            // if registration break
            if now.hour() == registration_hour && now.minute() == registration_time.1 {
                break;
            } else if now.hour() == registration_hour
                && now.minute() == registration_time.1 - 1
                && now.second() >= 50
            {
                // if 10 seconds off stop sleeping
                continue;
            } else {
                // if far away sleep
                sleep(Duration::from_secs(4)).await;
            }
        }
        pb.finish_with_message(format!(
            "Reloaded for registration at {}.",
            Local::now().format("%H:%M:%S.%3f")
        ));

        page.reload().await?.wait_for_navigation().await?;

        println!(
            "Page finished loading at {}",
            Local::now().format("%H:%M:%S.%3f")
        );
        let pb = get_progress_bar("Selecting courses...");
        for (index, checkbox) in wait_elements_agressive_retry(&page, elements.checkboxes, TIMEOUT)
            .await?
            .into_iter()
            .enumerate()
        {
            if selected_courses
                .iter()
                .any(|course| course.checkbox_index == index as u8)
            {
                checkbox.click().await?;
            }
        }
        pb.finish_with_message("Courses selected.");

        // enroll
        wait_element_agressive_retry(&page, elements.enroll_button, TIMEOUT)
            .await?
            .click()
            .await?;

        println!(
            "Enroll clicked at {}",
            Local::now().format("%H:%M:%S.%3f").to_string()
        );

        // confirm
        wait_element_agressive_retry(&page, &elements.enroll_confirm_button, TIMEOUT)
            .await?
            .click()
            .await?;

        println!(
            "Confirm clicked at {}",
            Local::now().format("%H:%M:%S.%3f").to_string()
        );

        // results
        let pb = get_progress_bar("Waiting for enrollment results...");
        wait_element_agressive_retry(&page, elements.results_rows, TIMEOUT).await?;
        let registration_results = elements.get_registration_results(&page).await?;
        pb.finish_with_message(format!(
            "Found {} enrollment results.",
            registration_results.len()
        ));
        println!("{}", registration_results.to_table());
    } else {
        let pb = get_progress_bar("Selecting courses...");
        for (index, checkbox) in wait_elements_agressive_retry(&page, elements.checkboxes, TIMEOUT)
            .await?
            .into_iter()
            .enumerate()
        {
            if selected_courses
                .iter()
                .any(|course| course.checkbox_index == index as u8)
            {
                checkbox.click().await?;
            }
        }
        pb.finish_with_message("Courses selected.");

        // validate
        wait_element_agressive_retry(&page, elements.validate_button, TIMEOUT)
            .await?
            .click()
            .await?;

        println!(
            "Validation clicked at {}",
            Local::now().format("%H:%M:%S.%3f").to_string()
        );
        // results
        let pb = get_progress_bar("Waiting for validation results...");
        wait_element_agressive_retry(&page, elements.results_rows, TIMEOUT).await?;
        let registration_results = elements.get_registration_results(&page).await?;
        pb.finish_with_message(format!(
            "Found {} validation results.",
            registration_results.len()
        ));
        println!("{}", registration_results.to_table());
    }

    Ok(())
}

enum CartTransition {
    In,
    Select,
}

enum AuthTransition {
    AuthSuccess,
    Duo,
    AuthFail,
}

enum DuoTransition {
    TimeOut,
    Trust,
    Cart,
}

async fn authentication_transition(
    page: &Page,
    elements: &EmoryPageElements,
    wait_time: u64,
) -> Result<AuthTransition, CdpError> {
    let start = Instant::now();
    let wait_time = Duration::new(wait_time, 0);
    loop {
        match page.find_element(elements.login_error).await {
            Ok(_) => return Ok(AuthTransition::AuthFail),
            Err(e) => {
                if start.elapsed() >= wait_time {
                    return Err(e);
                }
            }
        }
        match page.find_element(elements.duo_waiting).await {
            Ok(_) => return Ok(AuthTransition::Duo),
            Err(e) => {
                if start.elapsed() >= wait_time {
                    return Err(e);
                }
            }
        }
        match page.find_element(elements.semester_cart).await {
            Ok(_) => return Ok(AuthTransition::AuthSuccess),
            Err(e) => {
                if start.elapsed() >= wait_time {
                    return Err(e);
                }
            }
        }
        match page.find_element(elements.course_row).await {
            Ok(_) => return Ok(AuthTransition::AuthSuccess),
            Err(e) => {
                if start.elapsed() >= wait_time {
                    return Err(e);
                }
            }
        }
        sleep(Duration::from_millis(100)).await;
    }
}

async fn duo_transition(
    page: &Page,
    elements: &EmoryPageElements,
    wait_time: u64,
) -> Result<DuoTransition, CdpError> {
    let start = Instant::now();
    let wait_time = Duration::new(wait_time, 0);
    let mut code_announced = false;
    loop {
        if !code_announced {
            match page.find_element(elements.duo_verification_code).await {
                Ok(element) => {
                    if let Some(code_text) = element.inner_text().await? {
                        let code = code_text.trim();
                        if !code.is_empty() {
                            println!("Duo verification code: {}", code);
                            println!("Enter this code in Duo Mobile to approve the login.");
                            code_announced = true;
                        }
                    }
                }
                Err(_) => {}
            }
        }
        match page.find_element(elements.duo_trust_browser).await {
            Ok(element) => {
                element.click().await?;
                return Ok(DuoTransition::Trust);
            }
            Err(e) => {
                if start.elapsed() >= wait_time {
                    return Err(e);
                }
            }
        }
        match page.find_element(elements.duo_time_out_try_again).await {
            Ok(_) => return Ok(DuoTransition::TimeOut),
            Err(e) => {
                if start.elapsed() >= wait_time {
                    return Err(e);
                }
            }
        }
        match page.find_element(elements.semester_cart).await {
            Ok(_) => return Ok(DuoTransition::Cart),
            Err(e) => {
                if start.elapsed() >= wait_time {
                    return Err(e);
                }
            }
        }
        match page.find_element(elements.course_row).await {
            Ok(_) => return Ok(DuoTransition::Cart),
            Err(e) => {
                if start.elapsed() >= wait_time {
                    return Err(e);
                }
            }
        }
        sleep(Duration::from_millis(100)).await;
    }
}

async fn cart_transition(
    page: &Page,
    elements: &EmoryPageElements,
    wait_time: u64,
) -> Result<CartTransition, CdpError> {
    let start = Instant::now();
    let wait_time = Duration::new(wait_time, 0);
    loop {
        match page.find_element(elements.semester_cart).await {
            Ok(_) => return Ok(CartTransition::Select),
            Err(e) => {
                if start.elapsed() >= wait_time {
                    return Err(e);
                }
            }
        }
        match page.find_element(elements.course_row).await {
            Ok(_) => return Ok(CartTransition::In),
            Err(e) => {
                if start.elapsed() >= wait_time {
                    return Err(e);
                }
            }
        }
        sleep(Duration::from_millis(100)).await;
    }
}

async fn wait_element_agressive_retry(
    page: &Page,
    selector: &str,
    wait_time: u64,
) -> Result<Element, CdpError> {
    let start = Instant::now();
    let wait_time = Duration::new(wait_time, 0);
    loop {
        match page.find_element(selector).await {
            Ok(element) => return Ok(element),
            Err(e) => {
                if start.elapsed() < wait_time {
                    continue;
                } else {
                    return Err(e);
                }
            }
        }
    }
}

async fn wait_elements_agressive_retry(
    page: &Page,
    selector: &str,
    wait_time: u64,
) -> Result<Vec<Element>, CdpError> {
    let start = Instant::now();
    let wait_time = Duration::new(wait_time, 0);
    loop {
        match page.find_elements(selector).await {
            Ok(element) => return Ok(element),
            Err(e) => {
                if start.elapsed() < wait_time {
                    continue;
                } else {
                    return Err(e);
                }
            }
        }
    }
}

struct RegistrationTime(u32, u32, bool);

impl fmt::Display for RegistrationTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:02}:{:02} {}",
            self.0,
            self.1,
            if self.2 { "AM" } else { "PM" }
        )
    }
}

fn get_progress_bar(msg: impl Into<Cow<'static, str>>) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_style(
        ProgressStyle::with_template("{spinner:.blue} {msg}")
            .unwrap()
            .tick_strings(ascii::SPINNER),
    );
    pb.set_message(msg);
    pb
}

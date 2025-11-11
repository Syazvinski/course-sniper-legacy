use chromiumoxide::{error::CdpError, Element, Page};
use comfy_table::{Attribute, Cell, Color, Table};
use std::fmt;

#[derive(Debug, Clone)]
pub struct EmoryPageElements {
    pub page_url: &'static str,
    pub username_input: &'static str,
    pub passwd_input: &'static str,
    pub login_error: &'static str,
    pub validate_button: &'static str,
    pub enroll_button: &'static str,
    pub enroll_confirm_button: &'static str,
    pub semester_cart: &'static str,
    pub course_row: &'static str,
    pub checkboxes: &'static str,
    pub availability: &'static str,
    pub description: &'static str,
    pub schedule: &'static str,
    pub room: &'static str,
    pub instructor: &'static str,
    pub credits: &'static str,
    pub seats: &'static str,
    pub results_rows: &'static str,
    pub result_description: &'static str,
    pub result_status: &'static str,
    pub registration_success: &'static str,
    pub registration_fail: &'static str,
    pub duo_waiting: &'static str,
    pub duo_trust_browser: &'static str,
    pub duo_time_out_try_again: &'static str,
    pub duo_verification_code: &'static str,
}

impl Default for EmoryPageElements {
    fn default() -> Self {
        Self {
            page_url: "https://saprod.emory.edu/psc/saprod_48/EMPLOYEE/SA/c/SSR_STUDENT_FL.SSR_SHOP_CART_FL.GBL",
            username_input: "input#userid",
            passwd_input: "input#pwd",
            login_error: "div#ptloginerrorcont",
            validate_button: "a#DERIVED_SSR_FL_SSR_VALIDATE_FL",
            enroll_button: "a#DERIVED_SSR_FL_SSR_ENROLL_FL",
            enroll_confirm_button: r#"a[id="\#ICYes"]"#,
            semester_cart: r#"a[id^="SSR_CART_TRM_FL_TERM_DESCR30$"]"#,
            course_row: r#"tr[id^="SSR_REGFORM_VW$0_row_"]"#,
            checkboxes: r#"input[type="checkbox"][id^="DERIVED_REGFRM1_SSR_SELECT$"]"#,
            availability: r#"span[id^="DERIVED_SSR_FL_SSR_AVAIL_FL$"]"#,
            description: r#"span[id^="DERIVED_SSR_FL_SSR_DESCR80$"]"#,
            schedule: r#"span[id^="DERIVED_REGFRM1_SSR_MTG_SCHED_LONG$"]"#,
            room: r#"span[id^="DERIVED_REGFRM1_SSR_MTG_LOC_LONG$"]"#,
            instructor: r#"span[id^="DERIVED_REGFRM1_SSR_INSTR_LONG$"]"#,
            credits: r#"span[id^="DERIVED_SSR_FL_SSR_UNITS_LBL$"]"#,
            seats: r#"span[id^="DERIVED_SSR_FL_SSR_DESCR50$"]"#,
            results_rows: r#"div[id^="win48div$ICField229_row$"]"#,
            result_description: r#"span[id^="DERIVED_REGFRM1_DESCRLONG$"]"#,
            result_status: r#"div[id^="win48divDERIVED_REGFRM1_SSR_STATUS_LONG$"]"#,
            registration_success: "/cs/saprod/cache/PS_CS_STATUS_SUCCESS_ICN_1.gif",
            registration_fail: "/cs/saprod/cache/PS_CS_STATUS_ERROR_ICN_1.gif",
            duo_waiting: "div#auth-view-wrapper:not(.auth-error)",
            duo_trust_browser: r#"button[id="trust-browser-button"]"#,
            duo_time_out_try_again: r#"button.try-again-button"#,
            duo_verification_code: "div.verification-code",
        }
    }
}

pub struct ShoppingCart {
    pub element: Element,
    pub text: String,
}

impl fmt::Display for ShoppingCart {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}

#[derive(Debug)]
pub enum CourseStatus {
    Waitlist { position: u32 },
    Open { available: u32, capacity: u32 },
    Closed,
}

impl fmt::Display for CourseStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CourseStatus::Closed => write!(f, "Closed"),
            CourseStatus::Waitlist { position } => write!(f, "Waitlist {}", position),
            CourseStatus::Open {
                available,
                capacity,
            } => write!(f, "Open {}/{}", available, capacity),
        }
    }
}

#[derive(Debug)]
pub struct Course {
    pub checkbox_index: u8,
    pub availability: CourseStatus,
    pub description: String,
    pub schedule: String,
    pub room: String,
    pub instructor: String,
    pub credits: String,
}

impl fmt::Display for Course {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}

impl EmoryPageElements {
    pub async fn get_shopping_carts(&self, page: &Page) -> Result<Vec<ShoppingCart>, CdpError> {
        let semester_cart_elements = page.find_elements(self.semester_cart).await?;
        let semester_carts: Vec<ShoppingCart> =
            futures::future::join_all(semester_cart_elements.into_iter().map(|cart| async move {
                let text = cart.inner_text().await.unwrap().expect("test");
                ShoppingCart {
                    element: cart,
                    text,
                }
            }))
            .await;
        Ok(semester_carts)
    }

    pub async fn get_cart_courses(&self, page: &Page) -> Result<Vec<Course>, CdpError> {
        let course_row_elements = page.find_elements(self.course_row).await?;
        let courses: Vec<Course> =
            futures::future::try_join_all(course_row_elements.into_iter().enumerate().map(
                |(index, row)| async move {
                    let nums: Vec<u32> = row
                        .find_element(self.seats)
                        .await?
                        .inner_text()
                        .await?
                        .unwrap_or("".to_string())
                        .split_whitespace()
                        .filter_map(|word| word.parse().ok())
                        .collect();

                    let course_status = match row
                        .find_element(self.availability)
                        .await?
                        .inner_text()
                        .await?
                        .unwrap_or("".to_string())
                    {
                        text if text.contains("Wait List") => {
                            if nums.len() == 2 {
                                CourseStatus::Waitlist {
                                    position: nums[1] - nums[0],
                                }
                            } else {
                                CourseStatus::Waitlist { position: 999 }
                            }
                        }
                        text if text.contains("Closed") => CourseStatus::Closed,
                        text if text.contains("Open") => {
                            if nums.len() == 2 {
                                CourseStatus::Open {
                                    available: nums[0],
                                    capacity: nums[1],
                                }
                            } else {
                                CourseStatus::Open {
                                    available: 0,
                                    capacity: 0,
                                }
                            }
                        }
                        _ => CourseStatus::Closed,
                    };

                    Ok::<Course, CdpError>(Course {
                        checkbox_index: index as u8,
                        availability: course_status,
                        description: row
                            .find_element(self.description)
                            .await?
                            .inner_text()
                            .await?
                            .unwrap_or("None".to_string()),
                        schedule: row
                            .find_element(self.schedule)
                            .await?
                            .inner_text()
                            .await?
                            .unwrap_or("None".to_string()),
                        instructor: row
                            .find_element(self.instructor)
                            .await?
                            .inner_text()
                            .await?
                            .unwrap_or("None".to_string()),
                        room: row
                            .find_element(self.room)
                            .await?
                            .inner_text()
                            .await?
                            .unwrap_or("None".to_string()),
                        credits: row
                            .find_element(self.credits)
                            .await?
                            .inner_text()
                            .await?
                            .unwrap_or("None".to_string()),
                    })
                },
            ))
            .await?;

        Ok(courses)
    }

    pub async fn get_registration_results(
        &self,
        page: &Page,
    ) -> Result<Vec<RegistrationResult>, CdpError> {
        let result_elements = page.find_elements(self.results_rows).await?;
        let results: Vec<RegistrationResult> =
            futures::future::try_join_all(result_elements.into_iter().map(|result| async move {
                let status_html = result
                    .find_element(self.result_status)
                    .await?
                    .inner_html()
                    .await?
                    .unwrap_or("".to_string());
                Ok::<RegistrationResult, CdpError>(RegistrationResult {
                    description: result
                        .find_element(self.result_description)
                        .await?
                        .inner_text()
                        .await?
                        .unwrap_or("None".to_string()),
                    status: if status_html.contains(self.registration_success) {
                        RegistrationStatus::Success
                    } else if status_html.contains(self.registration_fail) {
                        RegistrationStatus::Fail
                    } else {
                        RegistrationStatus::Unknown
                    },
                })
            }))
            .await?;
        Ok(results)
    }
}

pub trait ToTable {
    fn to_table(&self) -> Table;
}

impl ToTable for Vec<Course> {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec![
            Cell::new("Course").add_attribute(Attribute::Bold),
            Cell::new("Credits"),
            Cell::new("Availability").add_attribute(Attribute::Bold),
            Cell::new("Schedule"),
            Cell::new("Room"),
            Cell::new("Instructor"),
        ]);

        for course in self {
            table.add_row(vec![
                Cell::new(course.description.clone()).fg(Color::Green),
                Cell::new(course.credits.clone()),
                Cell::new(course.availability.to_string()).fg(Color::Green),
                Cell::new(
                    course
                        .schedule
                        .split_whitespace()
                        .collect::<Vec<&str>>()
                        .join(" "),
                ),
                Cell::new(course.room.clone()),
                Cell::new(course.instructor.clone()),
            ]);
        }
        table
    }
}

pub enum RegistrationStatus {
    Success,
    Fail,
    Unknown,
}

impl fmt::Display for RegistrationStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &RegistrationStatus::Success => write!(f, "✅"),
            RegistrationStatus::Fail => write!(f, "❌"),
            RegistrationStatus::Unknown => write!(f, "❔"),
        }
    }
}

pub struct RegistrationResult {
    pub description: String,
    pub status: RegistrationStatus,
}

impl ToTable for Vec<RegistrationResult> {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec![Cell::new("Course"), Cell::new("Status")]);

        for result in self {
            table.add_row(vec![
                Cell::new(
                    result
                        .description
                        .split_whitespace()
                        .collect::<Vec<&str>>()
                        .join(" "),
                ),
                Cell::new(result.status.to_string())
                    .set_alignment(comfy_table::CellAlignment::Center),
            ]);
        }
        table
    }
}

use recite_config::ProducerRegistration;
use std::fmt::Write;
#[derive(Default)]
pub struct Panel {
    pub pages: Vec<String>,
    pub page: usize,
}
pub fn description(registration: &Result<Option<ProducerRegistration>, String>) -> Vec<String> {
    let text = match registration {
        Ok(Some(registration)) => {
            let command = registration.generate();
            let mut text = format!(
                "Executable: {:?}\nWorking directory: {:?}\nArguments (passed separately):",
                command.program(),
                command.directory()
            );
            for (index, argument) in command.args().iter().enumerate() {
                let _ = write!(text, "\n{}: {argument:?}", index + 1);
            }
            text
        }
        Err(error) => error.clone(),
        Ok(None) => {
            "No producer is registered. Generated declarations remain available for inspection."
                .into()
        }
    };
    super::super::text::pages(&text)
}

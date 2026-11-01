//! Cold project selection and destination preparation run before installing the owner.
use super::*;

pub(super) struct Loaded {
    pub(super) writer: Box<Writer>,
    pub(super) location: Result<(), String>,
}
pub(super) fn load(request: &Request) -> Result<Loaded, String> {
    request.checkpoint().map_err(|e| e.to_string())?;
    let location = request.options.location()?;
    let session = match &request.options.intent {
        Startup::Writer {
            project: Some(project),
            ..
        } => {
            let document = request.options.document.clone().or_else(|| {
                location
                    .as_ref()
                    .filter(|location| !location.document.is_empty())
                    .map(|location| location.document.clone().into())
            });
            Session::Project(Box::new(
                recite_writer_session::ProjectSession::open_with_control(
                    project,
                    document.as_deref(),
                    request,
                )
                .map_err(|e| e.to_string())?,
            ))
        }
        Startup::Examples { .. } => Session::examples().map_err(|e| e.to_string())?,
        _ => return Err("Choose a project or a temporary example.".into()),
    };
    let mut writer = Writer::from_session(session, request.options.editor.clone());
    if writer.session.project().is_some() {
        writer.map.personal_storage();
    }
    let location = location
        .as_ref()
        .map_or(Ok(()), |location| writer.follow_location(location));
    if let Err(error) = &location {
        if writer.session.dirty() {
            writer.status =
                format!("Recovered your draft; the linked location could not open: {error}");
        } else {
            return Err(error.clone());
        }
    }
    request.checkpoint().map_err(|e| e.to_string())?;
    Ok(Loaded {
        writer: Box::new(writer),
        location,
    })
}

#[cfg(test)]
mod tests;

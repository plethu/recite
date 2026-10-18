use super::worker::{Output, Work, Worker};
use crate::{editing::Writer, localisation::catalogue::CatalogueSave};

pub(super) async fn save(
    mut writer: Writer,
    request: Box<CatalogueSave>,
    worker: &Worker,
) -> Result<(), String> {
    let Output::Catalogue(mut request, result) = worker
        .execute(Work::Catalogue(request))
        .await
        .map_err(|e| e.to_string())?
    else {
        return Err("Invalid catalogue worker receipt.".into());
    };
    let accepted = result.and_then(|saved| {
        writer
            .localisation
            .write()
            .catalogue
            .as_mut()
            .ok_or("The catalogue is no longer open.")?
            .accept_save(&mut request, saved)
    });
    worker
        .execute(Work::RetireCatalogue(request))
        .await
        .map_err(|e| e.to_string())?;
    match accepted {
        Ok(Some(ticket)) => {
            let Output::Recovered(result) = worker
                .execute(Work::CatalogueRecovery(ticket))
                .await
                .map_err(|e| e.to_string())?
            else {
                return Err("Invalid catalogue recovery receipt.".into());
            };
            result.map_err(|e| e.to_string())
        }
        Ok(None) => Ok(()),
        Err(error) => Err(error),
    }
}

#include "recite.h"

int main() {
    ReciteStatus status = RECITE_OK;
    const auto request = RECITE_LOCALE_REQUEST_PLURAL;
    const auto domain = RECITE_LOCALE_DOMAIN_CHOICE;
    const auto outcome = RECITE_LOCALE_ATTEMPT_MISSING_TRANSLATION;
    return status == RECITE_STATUS_OK && RECITE_ERR_LOCALISATION == RECITE_STATUS_LOCALISATION
        && request == 1 && domain == 1 && outcome == 2 ? 0 : 1;
}

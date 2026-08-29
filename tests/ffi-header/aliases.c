#include "recite.h"

int main(void) {
    ReciteStatus status = RECITE_OK;
    uint32_t request = RECITE_LOCALE_REQUEST_SINGULAR;
    uint32_t domain = RECITE_LOCALE_DOMAIN_LINE;
    uint32_t outcome = RECITE_LOCALE_ATTEMPT_MATCHED;
    return status == RECITE_STATUS_OK && RECITE_ERR_LOCALISATION == RECITE_STATUS_LOCALISATION
        && request == 0 && domain == 0 && outcome == 3 ? 0 : 1;
}

#include "recite.h"

#include <stddef.h>

static ReciteConditionResult condition_callback(const ReciteConditionQuery *query, void *userdata) {
    (void)query;
    (void)userdata;
    ReciteConditionResult result = {0};
    return result;
}

static ReciteLocaleResult locale_callback(const ReciteLocaleQuery *query, void *userdata) {
    (void)query;
    (void)userdata;
    ReciteLocaleResult result = {0};
    return result;
}

int main(void) {
    uint64_t session = 0;
    ReciteBuffer batch = {0};
    const char locale[] = "fr";
    const char condition_name[] = "ready";
    const uint8_t snapshot[] = {0};

    if (recite_session_register_condition(0, condition_name, NULL, NULL)
        != RECITE_STATUS_VALIDATION) {
        return 1;
    }
    if (recite_session_register_condition(0, condition_name, condition_callback, NULL)
        != RECITE_STATUS_INVALID_HANDLE) {
        return 2;
    }
    if (recite_session_set_locale_provider(0, NULL, NULL) != RECITE_STATUS_VALIDATION) {
        return 3;
    }
    if (recite_session_set_locale_provider(0, locale_callback, NULL)
        != RECITE_STATUS_INVALID_HANDLE) {
        return 4;
    }
    if (recite_session_start_with_locale_provider(
            0, NULL, locale, NULL, NULL, &session, &batch)
        != RECITE_STATUS_VALIDATION
        || session != 0) {
        return 5;
    }
    if (recite_session_restore_with_values_and_locale_provider(
            0, snapshot, sizeof(snapshot), NULL, 0, NULL, NULL, &session, &batch)
        != RECITE_STATUS_VALIDATION
        || session != 0) {
        return 6;
    }
    return 0;
}

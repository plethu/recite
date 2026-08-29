#include "recite.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

enum { MAX_CALLBACK_RESULTS = 32 };

struct CallSlot {
    char text[64];
    char error[64];
    char matched_locale[32];
    char matched_context[96];
    char matched_key[32];
    char attempt_locale[32];
    char attempt_context[96];
    char attempt_key[32];
    ReciteLocaleAttempt attempt;
};

struct LifetimeOwner {
    struct CallSlot *slots;
    size_t used;
    unsigned int released_calls;
    int fail_next;
};

static int copy_string(char *destination, size_t capacity, const char *source) {
    size_t length;
    if (source == NULL || destination == NULL || capacity == 0) {
        return 0;
    }
    length = strlen(source);
    if (length >= capacity) {
        return 0;
    }
    memcpy(destination, source, length + 1);
    return 1;
}

static int begin_call(struct LifetimeOwner *owner) {
    if (owner->slots != NULL) {
        return 0;
    }
    owner->slots = calloc(MAX_CALLBACK_RESULTS, sizeof(*owner->slots));
    owner->used = 0;
    return owner->slots != NULL;
}

static int end_call(struct LifetimeOwner *owner) {
    if (owner->slots == NULL || owner->used > MAX_CALLBACK_RESULTS) {
        return 0;
    }
    free(owner->slots);
    owner->slots = NULL;
    owner->used = 0;
    owner->released_calls++;
    return 1;
}

static ReciteLocaleResult locale_callback(
    const ReciteLocaleQuery *query, void *userdata) {
    struct LifetimeOwner *owner = userdata;
    struct CallSlot *slot;
    ReciteLocaleResult result = {0};

    if (query == NULL || owner == NULL || owner->slots == NULL
        || owner->used >= MAX_CALLBACK_RESULTS) {
        return result;
    }
    slot = &owner->slots[owner->used++];
    if (owner->fail_next) {
        owner->fail_next = 0;
        if (!copy_string(slot->error, sizeof(slot->error), "owner error")) {
            return result;
        }
        result.error_message = slot->error;
        return (ReciteLocaleResult){0, NULL, -1, NULL, NULL, NULL, NULL, 0,
                                    slot->error};
    }

    if (query->kind == RECITE_LOCALE_REQUEST_KIND_PLURAL) {
        if (!copy_string(slot->text, sizeof(slot->text), "Deux lettres.")
            || !copy_string(slot->matched_locale, sizeof(slot->matched_locale), "fr")
            || !copy_string(slot->matched_context, sizeof(slot->matched_context), query->id)
            || !copy_string(slot->matched_key, sizeof(slot->matched_key), query->id)
            || !copy_string(slot->attempt_locale, sizeof(slot->attempt_locale), "fr")
            || !copy_string(slot->attempt_context, sizeof(slot->attempt_context), query->id)
            || !copy_string(slot->attempt_key, sizeof(slot->attempt_key), query->id)) {
            return result;
        }
        slot->attempt = (ReciteLocaleAttempt){
            slot->attempt_locale, slot->attempt_context, slot->attempt_key, 1,
            RECITE_LOCALE_ATTEMPT_OUTCOME_MATCHED};
        return (ReciteLocaleResult){
            1, slot->text, 1, slot->matched_locale, slot->matched_context,
            slot->matched_key, &slot->attempt, 1, NULL};
    }

    if (query->domain == RECITE_LOCALE_TEXT_DOMAIN_CHOICE) {
        if (!copy_string(slot->text, sizeof(slot->text), "Continuer.")) {
            return result;
        }
    } else if (!copy_string(slot->text, sizeof(slot->text), "Bonjour.")) {
        return result;
    }
    return (ReciteLocaleResult){1, slot->text, -1, NULL, NULL, NULL, NULL, 0,
                                NULL};
}

static int contains(const ReciteBuffer *buffer, const char *needle) {
    size_t needle_len = strlen(needle);
    size_t index;
    if (buffer == NULL || buffer->data == NULL || needle_len > buffer->len) {
        return 0;
    }
    for (index = 0; index + needle_len <= buffer->len; index++) {
        if (memcmp(buffer->data + index, needle, needle_len) == 0) {
            return 1;
        }
    }
    return 0;
}

static int read_asset(const char *path, unsigned char **bytes_out, size_t *len_out) {
    FILE *file;
    long length;
    unsigned char *bytes;
    size_t read_length;
    if (path == NULL || bytes_out == NULL || len_out == NULL) {
        return 0;
    }
    file = fopen(path, "rb");
    if (file == NULL || fseek(file, 0, SEEK_END) != 0) {
        if (file != NULL) fclose(file);
        return 0;
    }
    length = ftell(file);
    if (length <= 0 || fseek(file, 0, SEEK_SET) != 0) {
        fclose(file);
        return 0;
    }
    bytes = malloc((size_t)length);
    if (bytes == NULL) {
        fclose(file);
        return 0;
    }
    read_length = fread(bytes, 1, (size_t)length, file);
    fclose(file);
    if (read_length != (size_t)length) {
        free(bytes);
        return 0;
    }
    *bytes_out = bytes;
    *len_out = read_length;
    return 1;
}

static int check_status(ReciteStatus actual, ReciteStatus expected, int line) {
    if (actual == expected) {
        return 1;
    }
    fprintf(stderr, "line %d: expected status %d, got %d\n", line,
            (int)expected, (int)actual);
    return 0;
}

int main(int argc, char **argv) {
    struct LifetimeOwner owner = {0};
    const char count_name[] = "count";
    const char locale[] = "fr-CA";
    const char plural_header[] = "nplurals=2; plural=(n != 1);";
    const char choice_id[] = "81000000000000000003";
    char effect_id[512];
    ReciteInterpolationValue value;
    size_t nplurals = 0;
    uint64_t asset = 0;
    uint64_t unbegun_session = 0;
    uint64_t session = 0;
    ReciteBuffer batch = {0};
    ReciteBuffer prompt_snapshot = {0};
    unsigned char *asset_bytes = NULL;
    size_t asset_len = 0;

    if (argc != 3 || !read_asset(argv[1], &asset_bytes, &asset_len)
        || !check_status(recite_asset_load(asset_bytes, asset_len,
                                        &asset), RECITE_STATUS_OK, __LINE__)) {
        free(asset_bytes);
        return 1;
    }
    free(asset_bytes);
    if (snprintf(effect_id, sizeof(effect_id), "effect:%s:11:1#3", argv[2])
        < 0 || strlen(effect_id) >= sizeof(effect_id) - 1) {
        recite_asset_free(asset);
        return 2;
    }
    value = (ReciteInterpolationValue){count_name,
                                       RECITE_INTERPOLATION_VALUE_KIND_INTEGER,
                                       NULL, 2, 0.0, 0};
    if (!check_status(recite_locale_validate_plural_rule(plural_header, &nplurals),
                      RECITE_STATUS_OK, __LINE__) || nplurals != 2) {
        recite_asset_free(asset);
        return 2;
    }

    if (!check_status(recite_session_create_with_values(
                          asset, NULL, locale, &value, 1, &unbegun_session),
                      RECITE_STATUS_OK, __LINE__)
        || !check_status(recite_session_set_locale_provider(
                             unbegun_session, locale_callback, &owner),
                         RECITE_STATUS_OK, __LINE__)
        || !check_status(recite_session_snapshot(unbegun_session,
                                                 &prompt_snapshot),
                         RECITE_STATUS_OK, __LINE__)) {
        recite_session_free(unbegun_session);
        recite_asset_free(asset);
        return 3;
    }
    recite_session_free(unbegun_session);

    if (!begin_call(&owner)
        || !check_status(recite_session_start_with_values_and_locale_provider(
                             asset, NULL, locale, &value, 1, locale_callback,
                             &owner, &session, &batch), RECITE_STATUS_OK,
                         __LINE__)
        || !contains(&batch, "Bonjour.") || !contains(&batch, "Deux lettres.")
        || !contains(&batch, "Continuer.") || !end_call(&owner)) {
        return 4;
    }
    recite_buffer_free(&batch);

    if (!begin_call(&owner)
        || !check_status(recite_session_choose(session, choice_id, &batch),
                         RECITE_STATUS_OK, __LINE__)
        || !contains(&batch, "effect") || !end_call(&owner)) {
        return 5;
    }
    recite_buffer_free(&batch);

    owner.fail_next = 1;
    if (!begin_call(&owner)
        || !check_status(recite_session_acknowledge_effect(
                             session, effect_id, 1, NULL, &batch),
                         RECITE_STATUS_LOCALISATION, __LINE__)
        || !end_call(&owner)) {
        return 6;
    }
    if (!begin_call(&owner)
        || !check_status(recite_session_acknowledge_effect(
                             session, effect_id, 1, NULL, &batch),
                         RECITE_STATUS_OK, __LINE__)
        || !contains(&batch, "Bonjour.") || !end_call(&owner)) {
        return 7;
    }
    recite_buffer_free(&batch);
    recite_session_free(session);
    session = 0;

    if (!begin_call(&owner)
        || !check_status(recite_session_restore_with_values_and_locale_provider(
                             asset, prompt_snapshot.data, prompt_snapshot.len,
                             &value, 1, locale_callback, &owner, &session,
                             &batch), RECITE_STATUS_OK, __LINE__)
        || !contains(&batch, "Bonjour.") || !contains(&batch, "Deux lettres.")
        || !contains(&batch, "Continuer.")
        || !end_call(&owner)) {
        return 8;
    }
    recite_buffer_free(&batch);
    recite_buffer_free(&prompt_snapshot);
    if (session == 0) {
        return 9;
    }
    recite_session_free(session);
    recite_asset_free(asset);
    return owner.released_calls == 5 ? 0 : 10;
}

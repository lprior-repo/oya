#include "json.h"
#include <stdio.h>
#include <string.h>
#include <ctype.h>

static int escape_json_string(const char *input, char *output, size_t size) {
    size_t j = 0;
    for (size_t i = 0; input[i] && j < size - 1; i++) {
        char c = input[i];
        if (c == '"' || c == '\\') {
            if (j + 2 >= size) break;
            output[j++] = '\\';
            output[j++] = c;
        } else if (c == '\n') {
            if (j + 2 >= size) break;
            output[j++] = '\\';
            output[j++] = 'n';
        } else if (c == '\t') {
            if (j + 2 >= size) break;
            output[j++] = '\\';
            output[j++] = 't';
        } else {
            output[j++] = c;
        }
    }
    output[j] = '\0';
    return (int)j;
}

int json_pokemon_to_string(const Pokemon *p, char *buffer, size_t size) {
    if (!p || !buffer || size == 0) return -1;
    
    char escaped_name[MAX_NAME_LEN * 2];
    char escaped_type1[MAX_TYPE_LEN * 2];
    char escaped_type2[MAX_TYPE_LEN * 2];
    
    escape_json_string(p->name, escaped_name, sizeof(escaped_name));
    escape_json_string(p->type1, escaped_type1, sizeof(escaped_type1));
    escape_json_string(p->type2, escaped_type2, sizeof(escaped_type2));
    
    int len = snprintf(buffer, size,
        "{\n"
        "  \"id\": %d,\n"
        "  \"name\": \"%s\",\n"
        "  \"type1\": \"%s\",\n"
        "  \"type2\": \"%s\",\n"
        "  \"hp\": %d,\n"
        "  \"attack\": %d,\n"
        "  \"defense\": %d,\n"
        "  \"speed\": %d\n"
        "}",
        p->id, escaped_name, escaped_type1, escaped_type2,
        p->hp, p->attack, p->defense, p->speed);
    
    return len;
}

int json_pokemon_list_to_string(Pokemon *pokemons, size_t count, char *buffer, size_t size) {
    if (!pokemons || !buffer || size == 0) return -1;
    
    int offset = snprintf(buffer, size, "[\n");
    if (offset < 0 || (size_t)offset >= size) return -1;
    
    for (size_t i = 0; i < count; i++) {
        int written = json_pokemon_to_string(&pokemons[i], buffer + offset, size - offset);
        if (written < 0) return -1;
        offset += written;
        
        if (i < count - 1) {
            int comma = snprintf(buffer + offset, size - offset, ",\n");
            if (comma < 0 || (size_t)(offset + comma) >= size) return -1;
            offset += comma;
        }
    }
    
    int closing = snprintf(buffer + offset, size - offset, "\n]");
    if (closing < 0 || (size_t)(offset + closing) >= size) return -1;
    
    return offset + closing;
}

int json_error_to_string(const char *message, char *buffer, size_t size) {
    if (!message || !buffer || size == 0) return -1;
    return snprintf(buffer, size, "{\n  \"error\": \"%s\"\n}", message);
}

int json_health_to_string(const char *status, char *buffer, size_t size) {
    if (!status || !buffer || size == 0) return -1;
    return snprintf(buffer, size, "{\n  \"status\": \"%s\"\n}", status);
}

static void skip_whitespace(const char **json) {
    while (**json && isspace((unsigned char)**json)) (*json)++;
}

static int parse_string(const char **json, char *out, size_t max_len) {
    skip_whitespace(json);
    if (**json != '"') return -1;
    (*json)++;
    
    size_t i = 0;
    while (**json && **json != '"' && i < max_len - 1) {
        if (**json == '\\') {
            (*json)++;
            if (**json == 'n') out[i++] = '\n';
            else if (**json == 't') out[i++] = '\t';
            else if (**json == '"') out[i++] = '"';
            else if (**json == '\\') out[i++] = '\\';
            else if (**json) out[i++] = *(*json)++;
            else return -1;
        } else {
            out[i++] = *(*json)++;
        }
    }
    out[i] = '\0';
    if (**json == '"') (*json)++;
    return 0;
}

static int parse_number(const char **json, int *out) {
    skip_whitespace(json);
    if (!isdigit((unsigned char)**json)) return -1;
    
    int val = 0;
    while (isdigit((unsigned char)**json)) {
        val = val * 10 + (**json - '0');
        (*json)++;
    }
    *out = val;
    return 0;
}

int json_parse_pokemon(const char *json, Pokemon *pokemon) {
    if (!json || !pokemon) return -1;
    
    memset(pokemon, 0, sizeof(Pokemon));
    pokemon->id = -1;
    
    skip_whitespace(&json);
    if (*json != '{') return -1;
    json++;
    
    while (*json && *json != '}') {
        skip_whitespace(&json);
        if (*json == '}' || *json == ',') { json++; continue; }
        
        char key[64];
        if (parse_string(&json, key, sizeof(key)) != 0) return -1;
        
        skip_whitespace(&json);
        if (*json != ':') return -1;
        json++;
        skip_whitespace(&json);
        
        if (strcmp(key, "name") == 0) {
            if (parse_string(&json, pokemon->name, MAX_NAME_LEN) != 0) return -1;
        } else if (strcmp(key, "type1") == 0) {
            if (parse_string(&json, pokemon->type1, MAX_TYPE_LEN) != 0) return -1;
        } else if (strcmp(key, "type2") == 0) {
            if (parse_string(&json, pokemon->type2, MAX_TYPE_LEN) != 0) return -1;
        } else if (strcmp(key, "hp") == 0) {
            if (parse_number(&json, &pokemon->hp) != 0) return -1;
        } else if (strcmp(key, "attack") == 0) {
            if (parse_number(&json, &pokemon->attack) != 0) return -1;
        } else if (strcmp(key, "defense") == 0) {
            if (parse_number(&json, &pokemon->defense) != 0) return -1;
        } else if (strcmp(key, "speed") == 0) {
            if (parse_number(&json, &pokemon->speed) != 0) return -1;
        } else {
            if (*json == '"') { char tmp[256]; parse_string(&json, tmp, sizeof(tmp)); }
            else if (isdigit((unsigned char)*json)) { int tmp; parse_number(&json, &tmp); }
            else if (*json == '{') { int depth = 1; json++; while (*json && depth > 0) { if (*json == '{') depth++; if (*json == '}') depth--; json++; } }
            else if (*json == '[') { int depth = 1; json++; while (*json && depth > 0) { if (*json == '[') depth++; if (*json == ']') depth--; json++; } }
            else json++;
        }
    }
    
    if (pokemon->name[0] == '\0' || pokemon->type1[0] == '\0') return -1;
    return 0;
}

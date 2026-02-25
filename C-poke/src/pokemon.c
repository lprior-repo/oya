#include "pokemon.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <limits.h>

static int parse_int(const char *str, int *out) {
    if (!str || !*str) return -1;
    char *end;
    long val = strtol(str, &end, 10);
    if (*end != '\0' && *end != '\r' && *end != '\n') return -1;
    if (val < 0 || val > INT_MAX) return -1;
    *out = (int)val;
    return 0;
}

static char *trim(char *str) {
    while (*str == ' ' || *str == '\t') str++;
    char *end = str + strlen(str) - 1;
    while (end > str && (*end == ' ' || *end == '\t' || *end == '\r' || *end == '\n')) end--;
    *(end + 1) = '\0';
    return str;
}

static char *get_csv_field(char **line) {
    if (!line || !*line) return NULL;
    
    char *start = *line;
    char *end = strchr(start, ',');
    
    if (end) {
        *end = '\0';
        *line = end + 1;
    } else {
        end = start + strlen(start);
        *line = NULL;
    }
    
    return start;
}

static int parse_csv_line(char *line, Pokemon *pokemon) {
    char *field;
    
    field = get_csv_field(&line);
    if (!field) return -1;
    if (parse_int(trim(field), &pokemon->id) != 0) return -1;
    
    field = get_csv_field(&line);
    if (!field) return -1;
    strncpy(pokemon->name, trim(field), MAX_NAME_LEN - 1);
    pokemon->name[MAX_NAME_LEN - 1] = '\0';
    
    field = get_csv_field(&line);
    if (!field) return -1;
    strncpy(pokemon->type1, trim(field), MAX_TYPE_LEN - 1);
    pokemon->type1[MAX_TYPE_LEN - 1] = '\0';
    
    field = get_csv_field(&line);
    if (!field) return -1;
    strncpy(pokemon->type2, trim(field), MAX_TYPE_LEN - 1);
    pokemon->type2[MAX_TYPE_LEN - 1] = '\0';
    
    field = get_csv_field(&line);
    if (!field) return -1;
    if (parse_int(trim(field), &pokemon->hp) != 0) return -1;
    
    field = get_csv_field(&line);
    if (!field) return -1;
    if (parse_int(trim(field), &pokemon->attack) != 0) return -1;
    
    field = get_csv_field(&line);
    if (!field) return -1;
    if (parse_int(trim(field), &pokemon->defense) != 0) return -1;
    
    field = get_csv_field(&line);
    if (!field) return -1;
    if (parse_int(trim(field), &pokemon->speed) != 0) return -1;
    
    return 0;
}

int pokemon_store_init(PokemonStore *store, const char *csv_path) {
    if (!store || !csv_path) return -1;
    
    memset(store, 0, sizeof(PokemonStore));
    store->next_id = 1;
    
    FILE *file = fopen(csv_path, "r");
    if (!file) {
        fprintf(stderr, "Warning: Could not open %s: %s\n", csv_path, strerror(errno));
        return 0;
    }
    
    char line[512];
    int line_num = 0;
    int max_id = 0;
    
    while (fgets(line, sizeof(line), file)) {
        line_num++;
        
        if (line_num == 1) continue;
        
        size_t len = strlen(line);
        if (len == 0 || (len == 1 && line[0] == '\n')) continue;
        
        if (store->count >= MAX_POKEMON) {
            fprintf(stderr, "Warning: Maximum pokemon capacity reached\n");
            break;
        }
        
        if (parse_csv_line(line, &store->pokemons[store->count]) == 0) {
            if (store->pokemons[store->count].id > max_id) {
                max_id = store->pokemons[store->count].id;
            }
            store->count++;
        } else {
            fprintf(stderr, "Warning: Failed to parse line %d\n", line_num);
        }
    }
    
    store->next_id = max_id + 1;
    fclose(file);
    
    printf("Loaded %zu pokemon from %s\n", store->count, csv_path);
    return 0;
}

Pokemon *pokemon_find_by_id(PokemonStore *store, int id) {
    if (!store) return NULL;
    
    for (size_t i = 0; i < store->count; i++) {
        if (store->pokemons[i].id == id) {
            return &store->pokemons[i];
        }
    }
    return NULL;
}

int pokemon_add(PokemonStore *store, const Pokemon *pokemon) {
    if (!store || !pokemon) return -1;
    if (store->count >= MAX_POKEMON) return -1;
    
    store->pokemons[store->count] = *pokemon;
    store->pokemons[store->count].id = store->next_id++;
    store->count++;
    
    return store->pokemons[store->count - 1].id;
}

Pokemon *pokemon_get_all(PokemonStore *store, size_t *count) {
    if (!store || !count) return NULL;
    *count = store->count;
    return store->pokemons;
}

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "pokemon.h"
#include "server.h"

int main(int argc, char *argv[]) {
    int port = DEFAULT_PORT;
    const char *csv_path = "data/pokemon.csv";
    
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "-p") == 0 || strcmp(argv[i], "--port") == 0) {
            if (i + 1 < argc) {
                port = atoi(argv[++i]);
                if (port <= 0 || port > 65535) {
                    fprintf(stderr, "Invalid port number\n");
                    return 1;
                }
            }
        } else if (strcmp(argv[i], "-d") == 0 || strcmp(argv[i], "--data") == 0) {
            if (i + 1 < argc) {
                csv_path = argv[++i];
            }
        } else if (strcmp(argv[i], "-h") == 0 || strcmp(argv[i], "--help") == 0) {
            printf("Usage: %s [OPTIONS]\n", argv[0]);
            printf("Options:\n");
            printf("  -p, --port PORT    Server port (default: %d)\n", DEFAULT_PORT);
            printf("  -d, --data FILE    Path to CSV data file (default: data/pokemon.csv)\n");
            printf("  -h, --help         Show this help message\n");
            return 0;
        }
    }
    
    PokemonStore store;
    if (pokemon_store_init(&store, csv_path) != 0) {
        fprintf(stderr, "Failed to initialize Pokemon store\n");
        return 1;
    }
    
    Server server;
    if (server_init(&server, port, &store) != 0) {
        fprintf(stderr, "Failed to initialize server\n");
        return 1;
    }
    
    printf("Starting C-Poke API server on port %d\n", port);
    printf("Endpoints:\n");
    printf("  GET  /health        - Health check\n");
    printf("  GET  /pokemon       - List all Pokemon\n");
    printf("  GET  /pokemon/{id}  - Get Pokemon by ID\n");
    printf("  POST /pokemon       - Create new Pokemon\n");
    printf("\n");
    
    server_start(&server);
    server_stop(&server);
    
    printf("Server stopped\n");
    return 0;
}

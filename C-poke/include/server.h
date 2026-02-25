#ifndef SERVER_H
#define SERVER_H

#include "pokemon.h"

#define DEFAULT_PORT 8081
#define BACKLOG 10
#define BUFFER_SIZE 8192

typedef struct {
    int port;
    int socket_fd;
    PokemonStore *store;
    int running;
} Server;

int server_init(Server *server, int port, PokemonStore *store);
int server_start(Server *server);
void server_stop(Server *server);
int server_handle_request(Server *server, int client_fd);

#endif

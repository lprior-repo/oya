#include "server.h"
#include "http.h"
#include "json.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <signal.h>
#include <errno.h>

static Server *g_server = NULL;

static void signal_handler(int sig) {
    (void)sig;
    if (g_server) {
        printf("\nShutting down server...\n");
        g_server->running = 0;
    }
}

static int extract_id_from_path(const char *path) {
    if (strncmp(path, "/pokemon/", 9) != 0) return -1;
    const char *id_str = path + 9;
    if (*id_str == '\0') return -1;
    return atoi(id_str);
}

static void send_response(int client_fd, HttpResponse *res) {
    char buffer[MAX_RESPONSE_LEN + 1024];
    int len = http_response_serialize(res, buffer, sizeof(buffer));
    if (len > 0) {
        ssize_t sent = send(client_fd, buffer, len, 0);
        if (sent < 0) {
            perror("send failed");
        }
    }
}

static void handle_health(int client_fd) {
    HttpResponse res;
    http_response_init(&res, 200, "OK");
    http_response_set_header(&res, "Content-Type", "application/json");
    
    char body[128];
    json_health_to_string("healthy", body, sizeof(body));
    http_response_set_body(&res, body);
    
    send_response(client_fd, &res);
}

static void handle_get_pokemon(Server *server, int client_fd, const char *path) {
    HttpResponse res;
    char body[MAX_RESPONSE_LEN];
    
    if (strcmp(path, "/pokemon") == 0) {
        size_t count;
        Pokemon *pokemons = pokemon_get_all(server->store, &count);
        
        http_response_init(&res, 200, "OK");
        http_response_set_header(&res, "Content-Type", "application/json");
        json_pokemon_list_to_string(pokemons, count, body, sizeof(body));
        http_response_set_body(&res, body);
    } else {
        int id = extract_id_from_path(path);
        if (id <= 0) {
            http_response_init(&res, 400, "Bad Request");
            http_response_set_header(&res, "Content-Type", "application/json");
            json_error_to_string("Invalid Pokemon ID", body, sizeof(body));
            http_response_set_body(&res, body);
        } else {
            Pokemon *pokemon = pokemon_find_by_id(server->store, id);
            if (pokemon) {
                http_response_init(&res, 200, "OK");
                http_response_set_header(&res, "Content-Type", "application/json");
                json_pokemon_to_string(pokemon, body, sizeof(body));
                http_response_set_body(&res, body);
            } else {
                http_response_init(&res, 404, "Not Found");
                http_response_set_header(&res, "Content-Type", "application/json");
                json_error_to_string("Pokemon not found", body, sizeof(body));
                http_response_set_body(&res, body);
            }
        }
    }
    
    send_response(client_fd, &res);
}

static void handle_post_pokemon(Server *server, int client_fd, HttpRequest *req) {
    HttpResponse res;
    char body[MAX_RESPONSE_LEN];
    
    Pokemon pokemon;
    memset(&pokemon, 0, sizeof(pokemon));
    
    if (json_parse_pokemon(req->body, &pokemon) != 0) {
        http_response_init(&res, 400, "Bad Request");
        http_response_set_header(&res, "Content-Type", "application/json");
        json_error_to_string("Invalid JSON payload", body, sizeof(body));
        http_response_set_body(&res, body);
        send_response(client_fd, &res);
        return;
    }
    
    int id = pokemon_add(server->store, &pokemon);
    if (id < 0) {
        http_response_init(&res, 500, "Internal Server Error");
        http_response_set_header(&res, "Content-Type", "application/json");
        json_error_to_string("Failed to add Pokemon", body, sizeof(body));
        http_response_set_body(&res, body);
    } else {
        Pokemon *added = pokemon_find_by_id(server->store, id);
        http_response_init(&res, 201, "Created");
        http_response_set_header(&res, "Content-Type", "application/json");
        json_pokemon_to_string(added, body, sizeof(body));
        http_response_set_body(&res, body);
    }
    
    send_response(client_fd, &res);
}

static void handle_404(int client_fd) {
    HttpResponse res;
    http_response_init(&res, 404, "Not Found");
    http_response_set_header(&res, "Content-Type", "application/json");
    
    char body[128];
    json_error_to_string("Endpoint not found", body, sizeof(body));
    http_response_set_body(&res, body);
    
    send_response(client_fd, &res);
}

static void handle_405(int client_fd) {
    HttpResponse res;
    http_response_init(&res, 405, "Method Not Allowed");
    http_response_set_header(&res, "Content-Type", "application/json");
    
    char body[128];
    json_error_to_string("Method not allowed", body, sizeof(body));
    http_response_set_body(&res, body);
    
    send_response(client_fd, &res);
}

int server_init(Server *server, int port, PokemonStore *store) {
    if (!server || !store) return -1;
    
    memset(server, 0, sizeof(Server));
    server->port = port;
    server->store = store;
    server->running = 0;
    server->socket_fd = -1;
    
    return 0;
}

int server_start(Server *server) {
    if (!server) return -1;
    
    server->socket_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (server->socket_fd < 0) {
        perror("socket failed");
        return -1;
    }
    
    int opt = 1;
    if (setsockopt(server->socket_fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt)) < 0) {
        perror("setsockopt failed");
        close(server->socket_fd);
        return -1;
    }
    
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = INADDR_ANY;
    addr.sin_port = htons(server->port);
    
    if (bind(server->socket_fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        perror("bind failed");
        close(server->socket_fd);
        return -1;
    }
    
    if (listen(server->socket_fd, BACKLOG) < 0) {
        perror("listen failed");
        close(server->socket_fd);
        return -1;
    }
    
    g_server = server;
    signal(SIGINT, signal_handler);
    signal(SIGTERM, signal_handler);
    
    server->running = 1;
    printf("Server listening on port %d\n", server->port);
    
    while (server->running) {
        struct sockaddr_in client_addr;
        socklen_t client_len = sizeof(client_addr);
        
        int client_fd = accept(server->socket_fd, (struct sockaddr *)&client_addr, &client_len);
        if (client_fd < 0) {
            if (errno == EINTR && !server->running) break;
            perror("accept failed");
            continue;
        }
        
        char buffer[BUFFER_SIZE];
        ssize_t received = recv(client_fd, buffer, sizeof(buffer) - 1, 0);
        
        if (received > 0) {
            buffer[received] = '\0';
            
            HttpRequest req;
            if (http_request_parse(&req, buffer, received) == 0) {
                printf("%s %s\n", req.method, req.path);
                
                if (strcmp(req.path, "/health") == 0) {
                    if (strcmp(req.method, "GET") == 0) {
                        handle_health(client_fd);
                    } else {
                        handle_405(client_fd);
                    }
                } else if (strncmp(req.path, "/pokemon", 8) == 0) {
                    if (strcmp(req.method, "GET") == 0) {
                        handle_get_pokemon(server, client_fd, req.path);
                    } else if (strcmp(req.method, "POST") == 0 && strcmp(req.path, "/pokemon") == 0) {
                        handle_post_pokemon(server, client_fd, &req);
                    } else {
                        handle_405(client_fd);
                    }
                } else {
                    handle_404(client_fd);
                }
            } else {
                HttpResponse res;
                http_response_init(&res, 400, "Bad Request");
                char body[128];
                json_error_to_string("Invalid HTTP request", body, sizeof(body));
                http_response_set_body(&res, body);
                http_response_set_header(&res, "Content-Type", "application/json");
                send_response(client_fd, &res);
            }
        }
        
        close(client_fd);
    }
    
    return 0;
}

void server_stop(Server *server) {
    if (server && server->socket_fd >= 0) {
        close(server->socket_fd);
        server->socket_fd = -1;
    }
}

#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <unistd.h>

#define DEFAULT_PORT 8080
#define DEFAULT_ROOT "./public"
#define RECV_BUF 8192
#define PATH_BUF 4096
#define HDR_BUF 1024
#define FILE_CHUNK 65536

static const char *mime_type(const char *path) {
  const char *dot = strrchr(path, '.');
  if (!dot)
    return "application/octet-stream";
  if (strcmp(dot, ".html") == 0)
    return "text/html; charset=utf-8";
  if (strcmp(dot, ".css") == 0)
    return "text/css; charset=utf-8";
  if (strcmp(dot, ".js") == 0)
    return "application/javascript; charset=utf-8";
  if (strcmp(dot, ".svg") == 0)
    return "image/svg+xml";
  if (strcmp(dot, ".json") == 0)
    return "application/json; charset=utf-8";
  return "application/octet-stream";
}

static int path_has_dotdot(const char *path) {
  const char *p = path;
  while (*p) {
    if (p[0] == '.' && p[1] == '.' &&
        (p == path || p[-1] == '/') &&
        (p[2] == '\0' || p[2] == '/'))
      return 1;
    p++;
  }
  return 0;
}

static void send_all(int fd, const char *buf, size_t len) {
  size_t sent = 0;
  while (sent < len) {
    ssize_t n = send(fd, buf + sent, len - sent, 0);
    if (n < 0) {
      if (errno == EINTR)
        continue;
      return;
    }
    if (n == 0)
      return;
    sent += (size_t)n;
  }
}

static void send_response(int fd, int status, const char *reason,
                          const char *ctype, const char *body, size_t body_len) {
  char hdr[HDR_BUF];
  int n = snprintf(hdr, sizeof(hdr),
                   "HTTP/1.1 %d %s\r\n"
                   "Content-Type: %s\r\n"
                   "Content-Length: %zu\r\n"
                   "Connection: close\r\n"
                   "\r\n",
                   status, reason, ctype, body_len);
  if (n < 0 || (size_t)n >= sizeof(hdr))
    return;
  send_all(fd, hdr, (size_t)n);
  if (body && body_len > 0)
    send_all(fd, body, body_len);
}

static void send_file(int fd, const char *filepath) {
  int file_fd = open(filepath, O_RDONLY);
  if (file_fd < 0) {
    const char *msg = "404 Not Found\n";
    send_response(fd, 404, "Not Found", "text/plain; charset=utf-8", msg,
                  strlen(msg));
    return;
  }

  struct stat st;
  if (fstat(file_fd, &st) < 0 || !S_ISREG(st.st_mode)) {
    close(file_fd);
    const char *msg = "404 Not Found\n";
    send_response(fd, 404, "Not Found", "text/plain; charset=utf-8", msg,
                  strlen(msg));
    return;
  }

  const char *ctype = mime_type(filepath);
  char hdr[HDR_BUF];
  int n = snprintf(hdr, sizeof(hdr),
                   "HTTP/1.1 200 OK\r\n"
                   "Content-Type: %s\r\n"
                   "Content-Length: %lld\r\n"
                   "Connection: close\r\n"
                   "\r\n",
                   ctype, (long long)st.st_size);
  if (n < 0 || (size_t)n >= sizeof(hdr)) {
    close(file_fd);
    return;
  }
  send_all(fd, hdr, (size_t)n);

  char chunk[FILE_CHUNK];
  ssize_t r;
  while ((r = read(file_fd, chunk, sizeof(chunk))) > 0)
    send_all(fd, chunk, (size_t)r);

  close(file_fd);
}

static void handle_client(int client, const char *docroot) {
  char buf[RECV_BUF];
  ssize_t total = 0;

  while (total < (ssize_t)sizeof(buf) - 1) {
    ssize_t n = recv(client, buf + total, sizeof(buf) - 1 - (size_t)total, 0);
    if (n < 0) {
      if (errno == EINTR)
        continue;
      return;
    }
    if (n == 0)
      break;
    total += n;
    buf[total] = '\0';
    if (strstr(buf, "\r\n\r\n"))
      break;
  }

  if (total <= 0)
    return;

  buf[total] = '\0';

  char method[16];
  char uri[PATH_BUF];
  if (sscanf(buf, "%15s %4095s", method, uri) != 2) {
    const char *msg = "400 Bad Request\n";
    send_response(client, 400, "Bad Request", "text/plain; charset=utf-8", msg,
                  strlen(msg));
    return;
  }

  if (strcmp(method, "GET") != 0) {
    const char *msg = "405 Method Not Allowed\n";
    send_response(client, 405, "Method Not Allowed", "text/plain; charset=utf-8",
                  msg, strlen(msg));
    return;
  }

  char *q = strchr(uri, '?');
  if (q)
    *q = '\0';

  if (uri[0] != '/' || path_has_dotdot(uri)) {
    const char *msg = "404 Not Found\n";
    send_response(client, 404, "Not Found", "text/plain; charset=utf-8", msg,
                  strlen(msg));
    return;
  }

  const char *rel = uri;
  if (strcmp(uri, "/") == 0)
    rel = "/index.html";

  char filepath[PATH_BUF];
  int pn = snprintf(filepath, sizeof(filepath), "%s%s", docroot, rel);
  if (pn < 0 || (size_t)pn >= sizeof(filepath)) {
    const char *msg = "404 Not Found\n";
    send_response(client, 404, "Not Found", "text/plain; charset=utf-8", msg,
                  strlen(msg));
    return;
  }

  send_file(client, filepath);
}

int main(int argc, char **argv) {
  int port = DEFAULT_PORT;
  const char *docroot = DEFAULT_ROOT;

  if (argc >= 2)
    port = atoi(argv[1]);
  if (argc >= 3)
    docroot = argv[2];

  if (port <= 0 || port > 65535) {
    fprintf(stderr, "invalid port\n");
    return 1;
  }

  int server = socket(AF_INET, SOCK_STREAM, 0);
  if (server < 0) {
    perror("socket");
    return 1;
  }

  int yes = 1;
  if (setsockopt(server, SOL_SOCKET, SO_REUSEADDR, &yes, sizeof(yes)) < 0) {
    perror("setsockopt");
    close(server);
    return 1;
  }

  struct sockaddr_in addr;
  memset(&addr, 0, sizeof(addr));
  addr.sin_family = AF_INET;
  addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
  addr.sin_port = htons((uint16_t)port);

  if (bind(server, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
    perror("bind");
    close(server);
    return 1;
  }

  if (listen(server, 16) < 0) {
    perror("listen");
    close(server);
    return 1;
  }

  printf("Draconic todo server listening on http://127.0.0.1:%d (root=%s)\n",
         port, docroot);
  fflush(stdout);

  for (;;) {
    struct sockaddr_in client_addr;
    socklen_t client_len = sizeof(client_addr);
    int client = accept(server, (struct sockaddr *)&client_addr, &client_len);
    if (client < 0) {
      if (errno == EINTR)
        continue;
      perror("accept");
      break;
    }
    handle_client(client, docroot);
    close(client);
  }

  close(server);
  return 0;
}

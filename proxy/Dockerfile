FROM nginx:alpine
RUN echo 'server { \
    listen 8080; \
    location / { \
        proxy_pass http://35.184.213.248:8080; \
        proxy_set_header Host $host; \
        proxy_set_header X-Real-IP $remote_addr; \
        proxy_hide_header Access-Control-Allow-Origin; \
        proxy_hide_header Access-Control-Allow-Methods; \
        proxy_hide_header Access-Control-Allow-Headers; \
        add_header Access-Control-Allow-Origin "*" always; \
        add_header Access-Control-Allow-Methods "GET, POST, OPTIONS" always; \
        add_header Access-Control-Allow-Headers "Content-Type" always; \
        if ($request_method = OPTIONS) { return 204; } \
    } \
}' > /etc/nginx/conf.d/default.conf
EXPOSE 8080

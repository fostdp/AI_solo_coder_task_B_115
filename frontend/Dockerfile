FROM nginx:1.27-alpine

RUN apk add --no-cache brotli

COPY . /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf

WORKDIR /usr/share/nginx/html

RUN find . -type f \( -name "*.js" -o -name "*.css" -o -name "*.html" -o -name "*.svg" -o -name "*.json" \) \
    -exec gzip -9 -k {} \; \
    -exec brotli -q 11 -k {} \;

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]

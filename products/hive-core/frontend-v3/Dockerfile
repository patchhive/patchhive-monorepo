FROM node:20-alpine AS builder
WORKDIR /app
COPY products/hive-core/frontend-v3 ./products/hive-core/frontend-v3
WORKDIR /app/products/hive-core/frontend-v3
ARG VITE_API_URL=http://127.0.0.1:8100
ENV VITE_API_URL=$VITE_API_URL
RUN npm ci
RUN npm run build
FROM nginxinc/nginx-unprivileged:stable-alpine
COPY --from=builder /app/products/hive-core/frontend-v3/dist /usr/share/nginx/html
COPY products/hive-core/frontend-v3/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 8080

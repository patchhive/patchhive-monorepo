FROM node:20-alpine AS builder
WORKDIR /app
COPY packages/product-shell ./packages/product-shell
COPY packages/ui-v3 ./packages/ui-v3
COPY products/release-sentry/frontend ./products/release-sentry/frontend
WORKDIR /app/products/release-sentry/frontend
ARG VITE_API_URL=http://127.0.0.1:8100/api/products/release-sentry
ENV VITE_API_URL=$VITE_API_URL
RUN npm ci
RUN npm run build
FROM nginxinc/nginx-unprivileged:stable-alpine
COPY --from=builder /app/products/release-sentry/frontend/dist /usr/share/nginx/html
COPY products/release-sentry/frontend/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 8080

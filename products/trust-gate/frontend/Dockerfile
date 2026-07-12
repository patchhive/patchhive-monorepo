FROM node:20-alpine AS builder
WORKDIR /app
COPY packages/product-shell ./packages/product-shell
COPY packages/ui-v3 ./packages/ui-v3
COPY products/trust-gate/frontend ./products/trust-gate/frontend
WORKDIR /app/products/trust-gate/frontend
ARG VITE_API_URL=http://127.0.0.1:8100/api/products/trust-gate
ENV VITE_API_URL=$VITE_API_URL
RUN npm ci
RUN npm run build
FROM nginxinc/nginx-unprivileged:stable-alpine
COPY --from=builder /app/products/trust-gate/frontend/dist /usr/share/nginx/html
COPY products/trust-gate/frontend/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 8080

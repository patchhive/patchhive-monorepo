FROM node:20-alpine AS builder
WORKDIR /app
COPY packages/product-shell ./packages/product-shell
COPY packages/ui-v3 ./packages/ui-v3
COPY products/review-bee/frontend ./products/review-bee/frontend
WORKDIR /app/products/review-bee/frontend
ARG VITE_API_URL=http://127.0.0.1:8100/api/products/review-bee
ENV VITE_API_URL=$VITE_API_URL
RUN npm ci
RUN npm run build
FROM nginxinc/nginx-unprivileged:stable-alpine
COPY --from=builder /app/products/review-bee/frontend/dist /usr/share/nginx/html
COPY products/review-bee/frontend/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 8080

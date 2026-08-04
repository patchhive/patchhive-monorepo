FROM node:20-alpine AS builder
WORKDIR /app
COPY packages/product-shell ./packages/product-shell
COPY packages/ui ./packages/ui
COPY products/__PRODUCT_SLUG__/frontend ./products/__PRODUCT_SLUG__/frontend
WORKDIR /app/products/__PRODUCT_SLUG__/frontend
ARG VITE_API_URL=/api/products/__PRODUCT_SLUG__
ENV VITE_API_URL=$VITE_API_URL
RUN npm install --prefer-online --no-audit --no-fund
RUN npm run build

FROM nginxinc/nginx-unprivileged:stable-alpine
COPY --from=builder /app/products/__PRODUCT_SLUG__/frontend/dist /usr/share/nginx/html
COPY products/__PRODUCT_SLUG__/frontend/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 8080

FROM node:20-alpine AS builder
WORKDIR /app
COPY packages/ai-models ./packages/ai-models
COPY packages/product-shell ./packages/product-shell
COPY packages/ui ./packages/ui
COPY products/repo-reaper/frontend ./products/repo-reaper/frontend
WORKDIR /app/products/repo-reaper/frontend
ARG VITE_API_URL=http://127.0.0.1:8100/api/products/repo-reaper
ENV VITE_API_URL=$VITE_API_URL
RUN npm ci
RUN npm run build
FROM nginxinc/nginx-unprivileged:stable-alpine
COPY --from=builder /app/products/repo-reaper/frontend/dist /usr/share/nginx/html
COPY products/repo-reaper/frontend/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 8080

FROM node:20-alpine AS builder
WORKDIR /app
COPY packages/product-shell ./packages/product-shell
COPY packages/ui-v2 ./packages/ui-v2
COPY products/repo-reaper/frontend-v2 ./products/repo-reaper/frontend-v2
WORKDIR /app/products/repo-reaper/frontend-v2
RUN if [ -f package-lock.json ]; then npm ci; else npm install; fi
RUN npm run build

FROM nginxinc/nginx-unprivileged:stable-alpine
COPY --from=builder /app/products/repo-reaper/frontend-v2/dist /usr/share/nginx/html
COPY products/repo-reaper/frontend-v2/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 8080

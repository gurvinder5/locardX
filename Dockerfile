# Stage 1: Development environment with Vite dev server
FROM node:20-alpine AS dev

WORKDIR /app

# Install dependencies
COPY frontend/package*.json ./
RUN npm install --legacy-peer-deps

# Copy frontend sources
COPY frontend/ ./

EXPOSE 1420
CMD ["npm", "run", "dev"]

# Stage 2: Build static assets
FROM dev AS builder
RUN npm run build

# Stage 3: Production static server with Nginx
FROM nginx:alpine AS prod

COPY --from=builder /app/dist /usr/share/nginx/html

# Configure Nginx for SPA routing on port 1420 and port 80
RUN printf 'server {\n    listen 1420;\n    listen 80;\n    server_name localhost;\n    location / {\n        root /usr/share/nginx/html;\n        index index.html;\n        try_files $uri $uri/ /index.html;\n    }\n}\n' > /etc/nginx/conf.d/default.conf

EXPOSE 80 1420
CMD ["nginx", "-g", "daemon off;"]

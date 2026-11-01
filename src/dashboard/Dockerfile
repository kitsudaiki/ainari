FROM node:24-bookworm@sha256:e7add2a490bb17d97b485b8641bd88455fff7839ade0f5eba493c3508124d413

# set workdir
WORKDIR /app

# Accept build-time variable
ENV VITE_MIKO_API_URL=http://localhost:11417

# install deps first (better caching)
COPY app/package.json app/package-lock.json ./
RUN npm ci

# copy rest of the code
COPY app .

# expose dev server port
EXPOSE 5173

# start vite dev server
CMD ["npm", "run", "dev", "--", "--host", "0.0.0.0"]

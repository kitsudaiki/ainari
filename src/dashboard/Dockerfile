FROM node:24-bookworm

# set workdir
WORKDIR /app

# Accept build-time variable
ENV VITE_MIKO_API_URL=http://localhost:11417

# install deps first (better caching)
COPY app/package.json app/package-lock.json* ./
RUN npm install

# copy rest of the code
COPY app .

# expose dev server port
EXPOSE 5173

# start vite dev server
CMD ["npm", "run", "dev", "--", "--host", "0.0.0.0"]

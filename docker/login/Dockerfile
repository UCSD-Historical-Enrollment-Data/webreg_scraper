FROM node:lts-slim

RUN apt-get update && apt-get install -y \
    curl \
    libgtk-3-dev \
    libnotify-dev \
    libgconf-2-4 \
    libnss3 \
    libxss1 \
    libasound2 \
    && rm -rf /var/lib/apt/lists/*

ADD https://github.com/UCSD-Historical-Enrollment-Data/webreg_scraper.git#docker:scripts/webregautoin /app
ADD https://github.com/UCSD-Historical-Enrollment-Data/webreg_scraper.git#docker:docker/login /

# COPY ../webreg_scraper/scripts/webregautoin/ /app

WORKDIR /app

# COPY --chmod=+x ./start.sh /start.sh

# RUN ["chmod", "+x", "/start.sh"]

# echo "Installing dependencies..."
# npm i -g typescript

# npm i

# echo "Compiling TypeScript..."
# npm run compile

# exec node out/index.js --port 3000
# EOF

RUN ["npm", "install", "-g", "typescript"]
RUN ["npm", "install"]
RUN ["npm", "run", "compile"]

ENTRYPOINT ["node", "out/index.js", "--port", "3000"]

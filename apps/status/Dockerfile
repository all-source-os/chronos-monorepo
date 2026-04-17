FROM valeriansaliou/vigil:v1.29.0

# Copy our config into the container
COPY config.cfg /etc/vigil.cfg

# Vigil listens on 8080 per our config
EXPOSE 8080

CMD ["vigil", "-c", "/etc/vigil.cfg"]

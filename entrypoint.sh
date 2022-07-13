CONTAINER_ALREADY_STARTED=".started"

# while !</dev/tcp/postgres/5432; do sleep 1; done;

set -e
host="$1"
shift
  
until PGPASSWORD=$POSTGRES_PASSWORD psql -h "$host" -U "$POSTGRES_USER" -c '\q'; do
  >&2 echo "Postgres is unavailable - sleeping"
  sleep 1
done
  
>&2 echo "Postgres is up - executing command"

if [ ! -e $CONTAINER_ALREADY_STARTED ]; then
    touch $CONTAINER_ALREADY_STARTED
    echo "-- migrating database... --"
    diesel migration run --database-url $DATABASE_URL
    echo "migration complete!"
fi
./validator
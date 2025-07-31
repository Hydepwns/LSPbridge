module github.com/example/myproject

go 1.20

require (
	github.com/gin-gonic/gin v1.9.0
	github.com/stretchr/testify v1.8.2
	github.com/spf13/viper v1.15.0
	github.com/go-redis/redis/v9 v9.0.2
)

require (
	github.com/davecgh/go-spew v1.1.1 // indirect
	github.com/pmezard/go-difflib v1.0.0 // indirect
)

replace github.com/example/oldlib => github.com/example/newlib v2.0.0

replace github.com/broken/lib => ./vendor/fixed-lib
EOF < /dev/null
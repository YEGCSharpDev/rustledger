The container **wont** run like a normal container, it makes all the rustledger commands available. Use it like a binary. 

Example
``` bash
docker run --rm -v "$(pwd):/data" preface8675/rustledger rledger-check /data/sample.beancount
```

Running this in the directory where sample.beancount is will execute the rledger-check

Since aliases have been added `bean-check`, `bean-query` and `bean-report` can be used too instead of their rledger equivalents.

``` bash
docker run --rm -v "$(pwd):/data" preface8675/rustledger bean-check /data/sample.beancount
```

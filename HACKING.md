# How to regenerate bindings

```
bindgen spdk/include/spdk/nvme.h --with-derive-default --whitelist-function "spdk_(env|nvme|.*alloc|free|mempool).*" \ 
        --whitelist-type "spdk_(env|nvme|mempool).*" --generate functions,types  -- -Ispdk/include > src/clib.rs
```

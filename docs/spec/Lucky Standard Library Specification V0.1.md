# Lucky Standard Library Specification
<img src="../../logo/logo128.png" alt="Lucky logo" width="64" align="right" />


**Version:** 0.1 Draft
**Status:** Technical Specification

---

# Part I -- Foundation Types

---

## 1. Bool

Represents a logical truth value.

**Literals:** `true`, `false`

| Method | Signature | Description |
|---|---|---|
| `not` | `() -> Bool` | Logical negation |
| `and` | `(Bool) -> Bool` | Logical AND |
| `or` | `(Bool) -> Bool` | Logical OR |
| `xor` | `(Bool) -> Bool` | Logical XOR |
| `to_string` | `() -> String` | `"true"` or `"false"` |
| `to_int` | `() -> Int` | 1 or 0 |

**Static:** `Bool.parse(s: String) -> Bool?`

---

## 2. Int

Signed 64-bit integer (-2^63 to 2^63-1).

**Literals:** `42`, `-1`, `0xFF`, `0b1010`, `1_000_000`

| Method | Signature | Description |
|---|---|---|
| `abs` | `() -> Int` | Absolute value |
| `signum` | `() -> Int` | -1, 0, or 1 |
| `to_float` | `() -> Float` | Convert |
| `to_string` | `() -> String` | Decimal |
| `to_string_radix` | `(Int) -> String` | Base-N |
| `clamp` | `(Int, Int) -> Int` | Clamp |
| `min` | `(Int) -> Int` | Minimum |
| `max` | `(Int) -> Int` | Maximum |
| `pow` | `(Int) -> Int` | Exponentiation |
| `is_even` | `() -> Bool` | |
| `is_odd` | `() -> Bool` | |
| `count_ones` | `() -> Int` | Popcount |
| `rotate_left` | `(Int) -> Int` | Bit rotate |
| `rotate_right` | `(Int) -> Int` | Bit rotate |

**Operators:** `+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `&`, `|`, `^`, `<<`, `>>`, `~`

**Static:** `Int.min_value`, `Int.max_value`, `Int.parse(s) -> Int?`, `Int.random(lo, hi) -> Int`

---

## 3. Float

IEEE 754 binary64.

**Literals:** `3.14`, `1.0e9`, `-2.5e-3`, `Inf`, `-Inf`, `NaN`

| Method | Signature | Description |
|---|---|---|
| `abs` | `() -> Float` | Absolute |
| `ceil` | `() -> Int` | Round up |
| `floor` | `() -> Int` | Round down |
| `round` | `() -> Int` | Nearest |
| `trunc` | `() -> Int` | Toward zero |
| `sqrt` | `() -> Float` | Square root |
| `cbrt` | `() -> Float` | Cube root |
| `sin` | `() -> Float` | Sine (rad) |
| `cos` | `() -> Float` | Cosine |
| `tan` | `() -> Float` | Tangent |
| `asin` | `() -> Float` | Arc sine |
| `acos` | `() -> Float` | Arc cosine |
| `atan` | `() -> Float` | Arc tangent |
| `atan2` | `(Float) -> Float` | atan2 |
| `ln` | `() -> Float` | Natural log |
| `log2` | `() -> Float` | Log base 2 |
| `log10` | `() -> Float` | Log base 10 |
| `exp` | `() -> Float` | e^x |
| `pow` | `(Float) -> Float` | Power |
| `clamp` | `(Float, Float) -> Float` | Clamp |
| `is_nan` | `() -> Bool` | |
| `is_infinite` | `() -> Bool` | |
| `is_finite` | `() -> Bool` | |
| `to_int` | `() -> Int` | Truncate |
| `to_string` | `() -> String` | |

**Static:** `Float.pi`, `Float.e`, `Float.tau`, `Float.epsilon`, `Float.infinity`, `Float.nan`, `Float.parse(s)`, `Float.is_approx(a, b, eps?)`

---

## 4. Decimal

Fixed-point (128-bit, 28 digits). For currency and exact computation.

**Literals:** `3.14d`, `99.99d`

**Methods:** `abs()`, `round(digits)`, `floor()`, `ceil()`, `trunc()`, `to_float()`, `to_int()`, `to_string()`, `clamp(lo, hi)`

**Operators:** `+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `>`, `<=`, `>=`

**Static:** `Decimal.zero`, `Decimal.one`, `Decimal.parse(s)`, `Decimal.from_int(i)`

---

## 5. String

Immutable UTF-8 string.

**Literals:** `"hello"`, `"""multi-line"""`, `"interpolated: \{value}"`

### Core Methods

| Method | Signature | Description |
|---|---|---|
| `len` | `() -> Int` | Byte length |
| `char_count` | `() -> Int` | Unicode chars |
| `is_empty` | `() -> Bool` | |
| `get` | `(Int) -> String?` | Char at index |
| `slice` | `(Int, Int) -> String` | Substring |
| `contains` | `(String) -> Bool` | Substring test |
| `starts_with` | `(String) -> Bool` | Prefix test |
| `ends_with` | `(String) -> Bool` | Suffix test |
| `find` | `(String) -> Int?` | First occurrence |
| `rfind` | `(String) -> Int?` | Last occurrence |
| `split` | `(String) -> List<String>` | Split |
| `split_lines` | `() -> List<String>` | Newline split |
| `split_whitespace` | `() -> List<String>` | Whitespace split |
| `trim` | `() -> String` | Trim |
| `trim_start` | `() -> String` | Trim leading |
| `trim_end` | `() -> String` | Trim trailing |
| `to_uppercase` | `() -> String` | Unicode upper |
| `to_lowercase` | `() -> String` | Unicode lower |
| `replace` | `(String, String) -> String` | Replace all |
| `replace_regex` | `(String, String) -> String` | Regex replace |
| `matches_regex` | `(String) -> Bool` | Regex test |
| `repeat` | `(Int) -> String` | Repeat N |
| `pad_start` | `(Int, String?) -> String` | Left pad |
| `pad_end` | `(Int, String?) -> String` | Right pad |
| `reverse` | `() -> String` | Reverse |
| `to_bytes` | `() -> Bytes` | UTF-8 encode |
| `hash` | `() -> String` | SHA256 hex |
| `levenshtein` | `(String) -> Int` | Edit distance |
| `similarity` | `(String) -> Float` | 0-1 score |

**Operators:** `+` (concat), `*` (repeat: `"ab" * 3`), `==`, `!=`, `<`, `>`

**Static:** `String.empty`, `String.join(list, sep)`, `String.format(fmt, ...args)`

---

## 6. Bytes

Immutable raw byte sequence.

**Literals:** `0xDEADBEEF`

| Method | Signature | Description |
|---|---|---|
| `len` | `() -> Int` | Byte count |
| `is_empty` | `() -> Bool` | |
| `get` | `(Int) -> Int?` | Byte at index |
| `slice` | `(Int, Int) -> Bytes` | Sub-slice |
| `to_string` | `() -> String?` | UTF-8 decode |
| `to_hex` | `() -> String` | Hex |
| `to_base64` | `() -> String` | Base64 |
| `find` | `(Bytes) -> Int?` | Find |
| `split` | `(Bytes) -> List<Bytes>` | Split |
| `hash` | `() -> String` | SHA256 |

**Static:** `Bytes.empty`, `Bytes.from_string(s)`, `Bytes.from_hex(s)`, `Bytes.from_base64(s)`

---

## 7. Char

Single Unicode scalar value.

**Literals:** `'a'`, `'\n'`, `'\u{1F600}'`

| Method | Signature | Description |
|---|---|---|
| `is_ascii` | `() -> Bool` | |
| `is_alphabetic` | `() -> Bool` | |
| `is_numeric` | `() -> Bool` | |
| `is_alphanumeric` | `() -> Bool` | |
| `is_whitespace` | `() -> Bool` | |
| `is_uppercase` | `() -> Bool` | |
| `is_lowercase` | `() -> Bool` | |
| `to_uppercase` | `() -> Char` | |
| `to_lowercase` | `() -> Char` | |
| `to_int` | `() -> Int?` | Digit value |
| `to_string` | `() -> String` | |

---

## 8. Time

Absolute UTC point, nanosecond precision.

**Literals:** `time("2026-01-15T09:30:00Z")`

| Method | Signature | Description |
|---|---|---|
| `year` | `() -> Int` | |
| `month` | `() -> Int` | 1-12 |
| `day` | `() -> Int` | 1-31 |
| `hour` | `() -> Int` | 0-23 |
| `minute` | `() -> Int` | 0-59 |
| `second` | `() -> Int` | 0-59 |
| `nanosecond` | `() -> Int` | |
| `weekday` | `() -> Int` | 1=Mon, 7=Sun |
| `add` | `(Duration) -> Time` | |
| `sub` | `(Duration) -> Time` | |
| `diff` | `(Time) -> Duration` | |
| `format` | `(String) -> String` | strftime |
| `to_iso8601` | `() -> String` | |
| `to_unix` | `() -> Int` | |
| `to_unix_ms` | `() -> Int` | |
| `to_unix_ns` | `() -> Int` | |

**Operators:** `+` (Duration), `-` (Duration), `-` (Time -> Duration), `==`, `!=`, `<`, `>`

**Static:** `Time.now()`, `Time.epoch()`, `Time.from_unix(s)`, `Time.from_unix_ms(ms)`, `Time.parse(s, fmt)`, `Time.parse_iso8601(s)`

---

## 9. Duration

Signed time span, nanosecond precision.

**Literals:** `30s`, `5m`, `2h`, `1d`, `500ms`, `100us`, `50ns`

| Method | Signature | Description |
|---|---|---|
| `to_nanos` | `() -> Int` | |
| `to_millis` | `() -> Int` | |
| `to_seconds` | `() -> Float` | |
| `to_minutes` | `() -> Float` | |
| `to_hours` | `() -> Float` | |
| `to_days` | `() -> Float` | |
| `abs` | `() -> Duration` | |
| `mul` | `(Int) -> Duration` | |
| `div` | `(Int) -> Duration` | |

**Operators:** `+`, `-`, `*` (Int), `/` (Int), `==`, `!=`, `<`, `>`

**Static:** `Duration.zero`, `Duration.from_nanos(n)`, `Duration.from_millis(n)`, `Duration.from_seconds(n)`, `Duration.parse(s)`

---

## 10. UUID, URI, Version, Path

### UUID

RFC 9562 identifier.

**Literals:** `uuid("550e8400-e29b-41d4-a716-446655440000")`

**Methods:** `to_string()`, `to_bytes()`, `version()`
**Static:** `UUID.v4()`, `UUID.v7()`, `UUID.nil`, `UUID.parse(s)`

### URI

RFC 3986 identifier.

**Literals:** `uri("https://example.com/path?q=1")`

**Methods:** `scheme()`, `host()`, `port()`, `path()`, `query()`, `fragment()`, `resolve(s)`, `to_string()`
**Static:** `URI.parse(s)`, `URI.from_file_path(p)`

### Version

Semantic version.

**Literals:** `version("1.2.3")`, `version("2.0.0-beta.1")`

**Methods:** `major()`, `minor()`, `patch()`, `pre()`, `build()`, `is_stable()`, `satisfies(range)`, `to_string()`
**Operators:** `==`, `!=`, `<`, `>`, `<=`, `>=`

### Path

Filesystem path.

**Methods:** `to_string()`, `parent()`, `file_name()`, `file_stem()`, `extension()`, `join(s)`, `is_absolute()`, `exists()`, `canonicalize()`, `components()`
**Static:** `Path.cwd()`, `Path.home()`, `Path.temp()`, `Path.parse(s)`

---

## 11. Special Values

```lucky
null       # intentional absence -- inhabits T?
unknown    # "not yet computed" -- inhabits T!
error(msg, recoverable?)   # runtime error value
```

---

## 12. Result\<T, E = Error\>

```lucky
type Result<T, E> = Success { value: T } | Failure { error: E }
```

| Method | Signature | Description |
|---|---|---|
| `is_success` | `() -> Bool` | |
| `is_failure` | `() -> Bool` | |
| `unwrap` | `() -> T` | Value or panic |
| `unwrap_or` | `(T) -> T` | Value or default |
| `unwrap_or_else` | `(fn(E) -> T) -> T` | Compute from error |
| `expect` | `(String) -> T` | Unwrap with msg |
| `map` | `(fn(T) -> U) -> Result<U,E>` | Transform ok |
| `map_err` | `(fn(E) -> F) -> Result<T,F>` | Transform err |
| `and_then` | `(fn(T) -> Result<U,E>) -> Result<U,E>` | Chain |
| `or_else` | `(fn(E) -> Result<T,F>) -> Result<T,F>` | Recover |
| `ok` | `() -> T?` | Value or null |
| `err` | `() -> E?` | Error or null |

**Static:** `Result.success(v)`, `Result.failure(e)`

---

## 13. Secret\<T\>

Redacted in logs/checkpoints/telemetry. Only accessible via `.reveal()`.

| Method | Signature | Description |
|---|---|---|
| `reveal` | `() -> T` | Access inner value |
| `map` | `(fn(T) -> U) -> Secret<U>` | Transform |

**Static:** `Secret.new(value)`

---

# Part II -- Collection Types

---

## 14. List\<T\>

Ordered, immutable, persistent (RRB vector). O(log N) approx O(1) for most ops.

**Literals:** `[1, 2, 3]`

### Core

| Method | Signature | Description |
|---|---|---|
| `len` | `() -> Int` | |
| `is_empty` | `() -> Bool` | |
| `get` | `(Int) -> T?` | Element at index |
| `first` | `() -> T?` | |
| `last` | `() -> T?` | |
| `slice` | `(Int, Int) -> List<T>` | Sub-list |
| `take` | `(Int) -> List<T>` | |
| `skip` | `(Int) -> List<T>` | |
| `append` | `(T) -> List<T>` | New list |
| `prepend` | `(T) -> List<T>` | |
| `concat` | `(List<T>) -> List<T>` | |
| `insert` | `(Int, T) -> List<T>` | |
| `remove` | `(Int) -> List<T>` | |
| `set` | `(Int, T) -> List<T>` | |
| `reverse` | `() -> List<T>` | |
| `sort` | `() -> List<T>` | Natural order |
| `sort_by` | `(fn(T,T)->Ordering) -> List<T>` | |
| `sort_by_key` | `(fn(T)->K) -> List<T>` | |
| `dedup` | `() -> List<T>` | Consecutive dupes |
| `unique` | `() -> List<T>` | All dupes |

### Transform

| Method | Signature | Description |
|---|---|---|
| `map` | `(fn(T)->U) -> List<U>` | |
| `filter` | `(fn(T)->Bool) -> List<T>` | |
| `filter_map` | `(fn(T)->U?) -> List<U>` | |
| `flat_map` | `(fn(T)->List<U>) -> List<U>` | |
| `reduce` | `(U, fn(U,T)->U) -> U` | Left fold |
| `fold` | `(U, fn(U,T)->U) -> U` | Alias |
| `fold_right` | `(U, fn(T,U)->U) -> U` | Right fold |

### Numeric (for Numeric T)

| Method | Signature | Description |
|---|---|---|
| `sum` | `() -> T` | Sum |
| `product` | `() -> T` | Product |
| `min` | `() -> T?` | Min element |
| `max` | `() -> T?` | Max element |

### Search

| Method | Signature | Description |
|---|---|---|
| `all` | `(fn(T)->Bool) -> Bool` | All match |
| `any` | `(fn(T)->Bool) -> Bool` | Any match |
| `find` | `(fn(T)->Bool) -> T?` | First match |
| `find_index` | `(fn(T)->Bool) -> Int?` | Index of match |
| `position` | `(T) -> Int?` | Index of value |
| `contains` | `(T) -> Bool` | Membership |
| `count` | `(fn(T)->Bool) -> Int` | Count matches |

### Grouping & Joining

| Method | Signature | Description |
|---|---|---|
| `join` | `(String) -> String` | Join strings |
| `zip` | `(List<U>) -> List<(T,U)>` | Pair |
| `unzip` | `() -> (List<A>, List<B>)` | Unzip pairs |
| `enumerate` | `() -> List<(Int,T)>` | Indexed |
| `chunks` | `(Int) -> List<List<T>>` | Fixed-size |
| `windows` | `(Int) -> List<List<T>>` | Sliding |
| `group_by` | `(fn(T)->K) -> Map<K,List<T>>` | Group |
| `partition` | `(fn(T)->Bool) -> (List<T>, List<T>)` | Split |
| `intersperse` | `(T) -> List<T>` | Insert between |
| `permutations` | `() -> List<List<T>>` | |
| `combinations` | `(Int) -> List<List<T>>` | Size N |

### Conversion

| Method | Signature | Description |
|---|---|---|
| `to_set` | `() -> Set<T>` | |
| `to_stream` | `() -> Stream<T>` | |

**Indexing:** `list[0]`, `list[1..5]`, `list[..3]`, `list[3..]`

**Static:** `List.empty()`, `List.of(...args)`, `List.range(start, end, step?)`, `List.repeat(value, n)`, `List.from_stream(s)`

---

## 15. Set\<T\>

Unordered, immutable unique elements. HAMT-backed.

**Literals:** `{1, 2, 3}`

| Method | Signature | Description |
|---|---|---|
| `len` | `() -> Int` | |
| `is_empty` | `() -> Bool` | |
| `contains` | `(T) -> Bool` | Membership |
| `insert` | `(T) -> Set<T>` | Add |
| `remove` | `(T) -> Set<T>` | Remove |
| `union` | `(Set<T>) -> Set<T>` | Union |
| `intersection` | `(Set<T>) -> Set<T>` | Intersection |
| `difference` | `(Set<T>) -> Set<T>` | Difference |
| `symmetric_difference` | `(Set<T>) -> Set<T>` | XOR |
| `is_subset` | `(Set<T>) -> Bool` | |
| `is_superset` | `(Set<T>) -> Bool` | |
| `is_disjoint` | `(Set<T>) -> Bool` | |
| `map` | `(fn(T)->U) -> Set<U>` | |
| `filter` | `(fn(T)->Bool) -> Set<T>` | |
| `to_list` | `() -> List<T>` | |
| `to_stream` | `() -> Stream<T>` | |

**Static:** `Set.empty()`, `Set.of(...args)`, `Set.from_list(list)`

---

## 16. Map\<K, V\>

Immutable, insertion-ordered key-value store. HAMT-backed.

**Literals:** `{"key": "value", "count": 42}`

| Method | Signature | Description |
|---|---|---|
| `len` | `() -> Int` | |
| `is_empty` | `() -> Bool` | |
| `get` | `(K) -> V?` | Value for key |
| `contains` | `(K) -> Bool` | Key exists |
| `insert` | `(K, V) -> Map<K,V>` | Insert/update |
| `remove` | `(K) -> Map<K,V>` | Remove |
| `update` | `(K, fn(V?)->V) -> Map<K,V>` | Upsert |
| `merge` | `(Map<K,V>) -> Map<K,V>` | Merge (later wins) |
| `merge_with` | `(Map<K,V>, fn(V,V)->V) -> Map<K,V>` | With resolver |
| `keys` | `() -> List<K>` | |
| `values` | `() -> List<V>` | |
| `entries` | `() -> List<(K,V)>` | |
| `map_values` | `(fn(V)->U) -> Map<K,U>` | |
| `map_keys` | `(fn(K)->L) -> Map<L,V>` | |
| `filter` | `(fn(K,V)->Bool) -> Map<K,V>` | |
| `filter_keys` | `(fn(K)->Bool) -> Map<K,V>` | |
| `filter_values` | `(fn(V)->Bool) -> Map<K,V>` | |
| `find_key` | `(fn(V)->Bool) -> K?` | |
| `default` | `(K, V) -> V` | Get or insert |
| `to_list` | `() -> List<(K,V)>` | |

**Indexing:** `map["key"]` (returns `V?`), `map["key"] ?| default`

**Static:** `Map.empty()`, `Map.of((k,v)...)`, `Map.from_list(pairs)`, `Map.from_keys(keys, value)`, `Map.group_by(list, fn)`

---

## 17. Queue\<T\> and Stack\<T\>

### Queue\<T\> -- Immutable FIFO

| Method | Signature | Description |
|---|---|---|
| `len` | `() -> Int` | |
| `is_empty` | `() -> Bool` | |
| `enqueue` | `(T) -> Queue<T>` | Add to back |
| `enqueue_all` | `(List<T>) -> Queue<T>` | |
| `dequeue` | `() -> (T?, Queue<T>)` | Remove from front |
| `peek` | `() -> T?` | Front element |
| `to_list` | `() -> List<T>` | |

**Static:** `Queue.empty()`, `Queue.from_list(list)`

### Stack\<T\> -- Immutable LIFO

| Method | Signature | Description |
|---|---|---|
| `len` | `() -> Int` | |
| `is_empty` | `() -> Bool` | |
| `push` | `(T) -> Stack<T>` | Add to top |
| `pop` | `() -> (T?, Stack<T>)` | Remove from top |
| `peek` | `() -> T?` | Top element |
| `to_list` | `() -> List<T>` | Top-to-bottom |

**Static:** `Stack.empty()`, `Stack.from_list(list)`

---

## 18. Graph\<N, E = ()\>

Immutable directed graph with structural sharing.

| Method | Signature | Description |
|---|---|---|
| `node_count` | `() -> Int` | |
| `edge_count` | `() -> Int` | |
| `add_node` | `(N) -> (NodeId, Graph<N,E>)` | |
| `remove_node` | `(NodeId) -> Graph<N,E>` | |
| `add_edge` | `(NodeId, NodeId, E) -> Graph<N,E>` | |
| `remove_edge` | `(NodeId, NodeId) -> Graph<N,E>` | |
| `get_node` | `(NodeId) -> N?` | |
| `get_edge` | `(NodeId, NodeId) -> E?` | |
| `neighbors` | `(NodeId) -> List<NodeId>` | Outgoing |
| `in_neighbors` | `(NodeId) -> List<NodeId>` | Incoming |
| `nodes` | `() -> List<(NodeId,N)>` | |
| `edges` | `() -> List<(NodeId,NodeId,E)>` | |
| `roots` | `() -> List<NodeId>` | No incoming |
| `leaves` | `() -> List<NodeId>` | No outgoing |
| `topological_sort` | `() -> List<NodeId>?` | |
| `is_acyclic` | `() -> Bool` | |
| `shortest_path` | `(NodeId, NodeId) -> List<NodeId>?` | Dijkstra |
| `strongly_connected` | `() -> List<List<NodeId>>` | SCCs |
| `transitive_closure` | `() -> Graph<N,Bool>` | |
| `map_nodes` | `(fn(N)->M) -> Graph<M,E>` | |
| `map_edges` | `(fn(E)->F) -> Graph<N,F>` | |
| `reverse` | `() -> Graph<N,E>` | |
| `subgraph` | `(List<NodeId>) -> Graph<N,E>` | |

**Static:** `Graph.empty()`, `Graph.from_edges(edges)`

---

## 19. Tree\<T\>

Rooted tree.

| Method | Signature | Description |
|---|---|---|
| `root` | `() -> T` | Root data |
| `children` | `() -> List<Tree<T>>` | |
| `is_leaf` | `() -> Bool` | |
| `depth` | `() -> Int` | |
| `node_count` | `() -> Int` | |
| `map` | `(fn(T)->U) -> Tree<U>` | Transform |
| `flatten` | `() -> List<T>` | Pre-order |
| `flatten_level_order` | `() -> List<T>` | BFS |
| `find` | `(fn(T)->Bool) -> T?` | DFS search |
| `filter` | `(fn(T)->Bool) -> Tree<T>?` | |
| `prune` | `(fn(T)->Bool) -> Tree<T>` | Remove subtrees |

**Static:** `Tree.leaf(value)`, `Tree.node(value, children)`

---

## 20. Stream\<T\>

Lazy, potentially infinite sequence.

### Sources (Static)

| Constructor | Description |
|---|---|
| `Stream.empty()` | Empty |
| `Stream.of(...args)` | From values |
| `Stream.from_list(list)` | From list |
| `Stream.from_set(set)` | From set |
| `Stream.from_fn(fn()->T?)` | Generator |
| `Stream.range(start, end, step?)` | Numeric range |
| `Stream.repeat(value)` | Infinite repeat |
| `Stream.cycle(list)` | Infinite cycle |
| `Stream.iterate(seed, fn)` | seed, f(seed), ... |
| `Stream.unfold(state, fn)` | General unfold |
| `Stream.concat(streams)` | Concatenate |
| `Stream.poll(interval, fn)` | Poll |
| `Stream.from_channel(ch)` | From channel |
| `Stream.from_event(topic)` | From event bus |

### Intermediate Operations (return Stream\<U\>)

| Operation | Signature | Description |
|---|---|---|
| `map` | `(fn(T)->U)` | Transform |
| `filter` | `(fn(T)->Bool)` | Keep |
| `filter_map` | `(fn(T)->U?)` | Filter+map |
| `flat_map` | `(fn(T)->Stream<U>)` | Flatten |
| `take` | `(Int)` | First N |
| `skip` | `(Int)` | Skip N |
| `take_while` | `(fn(T)->Bool)` | |
| `skip_while` | `(fn(T)->Bool)` | |
| `scan` | `(U, fn(U,T)->U)` | Running fold |
| `enumerate` | `()` | Indexed |
| `zip` | `(Stream<U>)` | Pair |
| `merge` | `(Stream<T>)` | Interleave |
| `chain` | `(Stream<T>)` | Append |
| `dedup` | `()` | Consec. dupes |
| `distinct` | `()` | All dupes |
| `chunks` | `(Int)` | Batch |
| `windows` | `(Int)` | Sliding |
| `buffer` | `(Int)` | Buffered |
| `debounce` | `(Duration)` | |
| `throttle` | `(Duration)` | |
| `sample` | `(Duration)` | |
| `delay` | `(Duration)` | |
| `inspect` | `(fn(T)->())` | Debug side-effect |

### Terminal Operations

| Operation | Signature | Description |
|---|---|---|
| `collect` | `() -> List<T>` | To list |
| `collect_set` | `() -> Set<T>` | To set |
| `fold` | `(U, fn(U,T)->U) -> U` | Reduce |
| `reduce` | `(fn(T,T)->T) -> T?` | No initial |
| `count` | `() -> Int` | |
| `sum` | `() -> T` | Numeric |
| `first` | `() -> T?` | |
| `last` | `() -> T?` | |
| `nth` | `(Int) -> T?` | |
| `find` | `(fn(T)->Bool) -> T?` | |
| `all` | `(fn(T)->Bool) -> Bool` | |
| `any` | `(fn(T)->Bool) -> Bool` | |
| `max` | `() -> T?` | |
| `min` | `() -> T?` | |
| `for_each` | `(fn(T)->()) -> ()` | Consume |
| `for_each_concurrent` | `(fn(T)->(), Int) -> ()` | Parallel |
| `partition` | `(fn(T)->Bool) -> (List<T>,List<T>)` | Split |
| `group_by` | `(fn(T)->K) -> Map<K,List<T>>` | Group |

---

## 21. Range

Half-open interval [start, end).

**Literals:** `0..10`, `0..=10`, `..10`, `0..`

| Method | Signature | Description |
|---|---|---|
| `start` | `() -> Int` | |
| `end` | `() -> Int?` | Null if unbounded |
| `is_inclusive` | `() -> Bool` | `..=` form |
| `contains` | `(Int) -> Bool` | |
| `len` | `() -> Int?` | |
| `to_stream` | `() -> Stream<Int>` | |
| `to_list` | `() -> List<Int>` | |

---

# Part III -- AI Primitives

---

## 22. Model

Represents a configured LLM backend.

**Declaration:**
```lucky
model Claude(
    provider = "anthropic",
    version = "claude-sonnet-4-20250514",
    temperature = 0.7,
    max_tokens = 4096,
)
```

| Method | Signature | Description |
|---|---|---|
| `complete` | `(String, CompleteOptions?) -> String` | Single-turn |
| `complete_stream` | `(String, CompleteOptions?) -> Stream<String>` | Streaming |
| `chat` | `(List<Message>, CompleteOptions?) -> Message` | Multi-turn |
| `chat_stream` | `(List<Message>, CompleteOptions?) -> Stream<Message>` | Streaming chat |
| `provider` | `() -> String` | Provider name |
| `version` | `() -> String` | Model version |
| `context_window` | `() -> Int` | Max context tokens |
| `cost_per_1k_prompt` | `() -> Float` | Cost per 1k prompt tokens (USD) |
| `cost_per_1k_completion` | `() -> Float` | Cost per 1k completion tokens (USD) |
| `supports_vision` | `() -> Bool` | |
| `supports_tools` | `() -> Bool` | |

**Related types:**
```lucky
type CompleteOptions
    temperature: Float?; max_tokens: Int?; top_p: Float?
    stop_sequences: List<String>; tools: List<ToolDef>
    response_format: ResponseFormat?

type Message
    role: "system" | "user" | "assistant" | "tool"
    content: String
    tool_calls: List<ToolCall>?; tool_call_id: String?

type ToolCall
    id: String; name: String; arguments: Map<String, Any>
```

---

## 23. Prompt

Structured prompt template. Validated at compile time.

**Declaration:**
```lucky
prompt CodeReviewer
    role
        You are a senior software engineer reviewing {language} code.
    rules
        - Report only actionable findings.
        - Classify severity: low, medium, high, critical.
    examples
        input: [bad code]
        output: [review]
    format
        Return JSON: { summary, findings[] }
```

| Method | Signature | Description |
|---|---|---|
| `render` | `(Map<String,Any>) -> String` | Render with variables |
| `role` | `() -> String?` | Role section |
| `rules` | `() -> List<String>` | Rules |
| `examples` | `() -> List<(String,String)>` | Example pairs |
| `format` | `() -> String?` | Format spec |
| `validate` | `() -> Result<(), List<String>>` | Validate sections |

---

## 24. Agent

Stateful AI entity owning memory, tools, prompt, and permissions.

**Declaration:**
```lucky
agent Researcher
    model Claude
    memory ResearchMemory
    tools [Browser, Search]
    permissions
        allow browser.search, http.get
        deny filesystem.write
    policy
        retry 2; timeout 5m
    prompt ResearchPrompt
```

| Method | Signature | Description |
|---|---|---|
| `name` | `() -> String` | Identifier |
| `model` | `() -> Model` | Bound model |
| `memory` | `() -> Memory` | Agent memory |
| `tools` | `() -> List<Tool>` | Available tools |
| `permissions` | `() -> PermissionSet` | Effective permissions |
| `policy` | `() -> Policy` | Execution policy |
| `prompt` | `() -> Prompt?` | System prompt |
| `invoke` | `(String, Map<String,Any>?) -> Result<Any>` | Invoke task |
| `ask` | `(String) -> String` | Direct LLM query |
| `new` | `(AgentConfig?) -> Agent` | Create instance |

---

## 25. Task\<I, O\>

Smallest schedulable, checkpointable unit of work.

**Declaration:**
```lucky
task AnalyzeRepo
    input
        repo: URI
        depth: Int = 1
    output
        report: Document
    policy
        timeout 10m
    steps
        clone repo
        analyze_structure
        generate report
```

| Method | Signature | Description |
|---|---|---|
| `name` | `() -> String` | Identifier |
| `input_schema` | `() -> Map<String,TypeRef>` | Input types |
| `output_schema` | `() -> Map<String,TypeRef>` | Output types |
| `run` | `(Map<String,Any>?) -> TaskResult<O>` | Synchronous execute |
| `run_async` | `(Map<String,Any>?) -> Promise<TaskResult<O>>` | Async execute |
| `status` | `() -> TaskStatus` | Current status |
| `cancel` | `() -> ()` | Cancel |

**TaskResult:** `Success { value } | Failure { error } | Cancelled | Skipped`
**TaskStatus:** `Created | Ready | Running | Waiting | Checkpointed | Completed | Failed | Cancelled | Skipped`

---

## 26. Workflow\<I, O\>

DAG orchestrating agents and tasks.

**Declaration:**
```lucky
workflow BuildAndDeploy
    context
        repo: URI
        environment: String = "staging"
    Research -> Plan ->
    parallel
        Implement
        WriteTests
    wait -> Review -> Deploy
```

| Method | Signature | Description |
|---|---|---|
| `name` | `() -> String` | Identifier |
| `start` | `(Map<String,Any>?) -> WorkflowRun` | Start |
| `context` | `() -> Map<String,Any>` | Default context |
| `graph` | `() -> Graph<String,()>` | DAG structure |
| `validate` | `() -> Result<(), List<String>>` | Validate |

### WorkflowRun

| Method | Signature | Description |
|---|---|---|
| `id` | `() -> UUID` | Run ID |
| `status` | `() -> RunStatus` | Current status |
| `progress` | `() -> Float` | 0.0-1.0 |
| `result` | `() -> Result<Map<String,Any>>` | Final result |
| `cancel` | `() -> ()` | Cancel |
| `wait` | `() -> Result<Map<String,Any>>` | Block until done |
| `events` | `() -> Stream<RunEvent>` | Event stream |

---

## 27. Goal

Highest abstraction -- defines what success means.

**Declaration:**
```lucky
goal BuildWebsite
    success
        website.online
        website.tested
    workflow MainWorkflow
    workflow QuickWorkflow
```

| Method | Signature | Description |
|---|---|---|
| `name` | `() -> String` | Identifier |
| `success_criteria` | `() -> List<Criterion>` | Conditions |
| `workflows` | `() -> List<Workflow>` | Available workflows |
| `select_workflow` | `(Map<String,Any>) -> Workflow` | Policy-based selection |
| `pursue` | `(Map<String,Any>?) -> GoalRun` | Start pursuit |

---

## 28. Memory

Persistent, queryable storage for agents.

**Declaration:**
```lucky
memory ProjectMemory
    scope project
    backend vector
    dimensions 1536
    metric cosine
```

| Method | Signature | Description |
|---|---|---|
| `name` | `() -> String` | Identifier |
| `scope` | `() -> MemoryScope` | local/session/project/org/global |
| `remember` | `(String, Any, Embedding?) -> ()` | Store |
| `recall` | `(String) -> Any?` | Retrieve by key |
| `forget` | `(String) -> ()` | Remove |
| `similar` | `(Embedding, Int) -> List<(String,Any,Float)>` | K-NN search |
| `similar_to` | `(String, Int) -> List<(String,Any,Float)>` | By key's embedding |
| `search` | `(String, Int) -> List<(String,Any,Float)>` | Full-text/hybrid |
| `list` | `(String?, List<String>?) -> List<String>` | List keys |
| `contains` | `(String) -> Bool` | Key check |
| `count` | `() -> Int` | Entry count |
| `clear` | `() -> ()` | Remove all |
| `export` | `() -> List<(String,Any)>` | Export |
| `import` | `(List<(String,Any)>) -> ()` | Import |

---

## 29. Knowledge

Structured domain knowledge for RAG.

**Declaration:**
```lucky
knowledge CompanyDocs
    source "./docs/**/*.md"
    source uri("https://docs.internal.example.com")
    chunk_size 1024
    chunk_overlap 128
    embedding_model "text-embedding-3-small"
```

| Method | Signature | Description |
|---|---|---|
| `name` | `() -> String` | Identifier |
| `sources` | `() -> List<Source>` | Sources |
| `search` | `(String, Int) -> List<Chunk>` | Semantic search |
| `ask` | `(String, Model?, Int) -> Answer` | RAG query |
| `index` | `() -> ()` | Rebuild index |
| `add_source` | `(Source) -> ()` | Add source |
| `remove_source` | `(URI) -> ()` | Remove source |

**Related types:**
```lucky
type Source
    kind: "file" | "directory" | "url"
    path: String; glob: String?; recursive: Bool

type Chunk
    id: String; content: String; source: URI
    start_line: Int; end_line: Int; score: Float

type Answer
    text: String; citations: List<Chunk>; confidence: Float
```

---

## 30. Context

Implicit, lexically-scoped execution state.

| Method | Signature | Description |
|---|---|---|
| `get` | `(String) -> Any?` | Get value |
| `get_typed` | `(String) -> T?` | Typed get |
| `has` | `(String) -> Bool` | Key exists |
| `keys` | `() -> List<String>` | All keys |
| `to_map` | `() -> Map<String,Any>` | Snapshot |
| `with_entry` | `(String, Any) -> Context` | New layer |
| `with_entries` | `(Map<String,Any>) -> Context` | New layer |
| `scope` | `() -> ScopeId` | Current scope |

---

## 31. Tool

Capability interface to external systems.

**Declaration:**
```lucky
tool GitCLI(
    workdir = "./repo",
    author_name = context.user.name,
)
```

| Method | Signature | Description |
|---|---|---|
| `name` | `() -> String` | Identifier |
| `invoke` | `(String, Map<String,Any>) -> Result<Any>` | Call method |
| `methods` | `() -> List<MethodDef>` | Available methods |
| `describe` | `() -> String` | For LLM function calling |
| `describe_method` | `(String) -> String` | Method description |
| `validate_args` | `(String, Map<String,Any>) -> Result<()>` | Validate args |

---

## 32. Probabilistic\<T\>

AI output with confidence metadata.

```lucky
type Probabilistic<T>
    value: T
    confidence: Float          # 0.0 to 1.0
    reasoning: String?
    citations: List<Citation>
    alternatives: List<(T, Float)>?
```

| Method | Signature | Description |
|---|---|---|
| `is_certain` | `(Float?) -> Bool` | confidence >= threshold |
| `is_likely` | `(Float?) -> Bool` | confidence >= threshold |
| `with_threshold` | `(Float) -> T?` | Value if confident |
| `best_alternative` | `() -> (T, Float)?` | Best alt |
| `map` | `(fn(T)->U) -> Probabilistic<U>` | Transform |
| `to_result` | `(Float?) -> Result<T>` | Convert |

---

# Part IV -- Standard Tools

---

## 33. Filesystem

Permission-gated file operations.

**Configuration:**
```lucky
tool Filesystem(
    root = "./project",
    allowed_paths = ["**"],
    denied_paths = [".git/**", "node_modules/**"],
)
```

| Method | Signature | Description |
|---|---|---|
| `read` | `(Path) -> Result<String>` | Read UTF-8 |
| `read_bytes` | `(Path) -> Result<Bytes>` | Read raw |
| `read_lines` | `(Path) -> Result<List<String>>` | Read lines |
| `write` | `(Path, String) -> Result<()>` | Write string |
| `write_bytes` | `(Path, Bytes) -> Result<()>` | Write bytes |
| `append` | `(Path, String) -> Result<()>` | Append |
| `exists` | `(Path) -> Bool` | Check existence |
| `is_file` | `(Path) -> Bool` | |
| `is_dir` | `(Path) -> Bool` | |
| `create_dir` | `(Path) -> Result<()>` | Create dir recursively |
| `list` | `(Path?) -> Result<List<Path>>` | List directory |
| `walk` | `(Path?, String?) -> Stream<Path>` | Walk + glob |
| `glob` | `(String) -> List<Path>` | Glob match |
| `remove` | `(Path) -> Result<()>` | Delete file |
| `remove_dir_all` | `(Path) -> Result<()>` | Delete dir |
| `rename` | `(Path, Path) -> Result<()>` | Move |
| `copy` | `(Path, Path) -> Result<()>` | Copy file |
| `metadata` | `(Path) -> Result<FileMetadata>` | File metadata |
| `size` | `(Path) -> Result<Int>` | File size |
| `modified_at` | `(Path) -> Result<Time>` | Last modified |
| `checksum` | `(Path) -> Result<String>` | SHA256 hash |
| `watch` | `(Path) -> Stream<FileEvent>` | Watch for changes |
| `temp_file` | `(String?, String?) -> Result<Path>` | Temp file |
| `temp_dir` | `(String?) -> Result<Path>` | Temp directory |

---

## 34. Git

Version control operations.

**Configuration:**
```lucky
tool Git(
    workdir = "./repo",
    author_name = "Lucky Agent",
    author_email = "agent@lucky.dev",
)
```

| Method | Signature | Description |
|---|---|---|
| `clone` | `(URI, Path?) -> Result<()>` | Clone repo |
| `init` | `(Path?) -> Result<()>` | Init repo |
| `status` | `(Path?) -> Result<GitStatus>` | Working tree |
| `diff` | `(String?, String?) -> Result<String>` | Diff |
| `diff_files` | `(String?, String?) -> Result<List<String>>` | Changed files |
| `add` | `(String|List<String>) -> Result<()>` | Stage |
| `commit` | `(String) -> Result<String>` | Commit |
| `push` | `(String?, String?) -> Result<()>` | Push |
| `pull` | `(String?, String?) -> Result<()>` | Pull |
| `fetch` | `(String?) -> Result<()>` | Fetch |
| `branch` | `(String) -> Result<()>` | Create branch |
| `checkout` | `(String) -> Result<()>` | Switch |
| `merge` | `(String) -> Result<()>` | Merge |
| `rebase` | `(String) -> Result<()>` | Rebase |
| `log` | `(Int?, String?) -> Result<List<Commit>>` | History |
| `show` | `(String) -> Result<String>` | Show commit |
| `blame` | `(Path, Int?, Int?) -> Result<List<BlameLine>>` | Blame |
| `tag` | `(String, String?) -> Result<()>` | Tag |
| `tags` | `() -> Result<List<String>>` | List tags |
| `stash` | `(String?) -> Result<()>` | Stash |
| `stash_pop` | `() -> Result<()>` | Pop stash |
| `current_branch` | `() -> Result<String>` | |
| `branches` | `() -> Result<List<String>>` | |
| `has_changes` | `() -> Bool` | Dirty tree |
| `create_pr` | `(CreatePROptions) -> Result<PR>` | Create PR |
| `list_prs` | `(String?) -> Result<List<PR>>` | List PRs |
| `get_pr` | `(Int) -> Result<PR>` | Get PR |
| `review_pr` | `(Int, String, String?) -> Result<()>` | Review PR |
| `merge_pr` | `(Int, String?) -> Result<()>` | Merge PR |

**Related types:**
```lucky
type GitStatus
    branch: String; ahead: Int; behind: Int
    staged: List<FileStatus>; unstaged: List<FileStatus>
    untracked: List<String>; is_clean: Bool

type FileStatus
    path: String; status: "added"|"modified"|"deleted"|"renamed"; old_path: String?

type Commit
    hash: String; message: String; author: String
    email: String; timestamp: Time; files_changed: Int

type BlameLine
    line_number: Int; commit_hash: String; author: String
    timestamp: Time; content: String

type CreatePROptions
    title: String; body: String?; base: String; head: String; draft: Bool?

type PR
    number: Int; title: String; body: String?
    state: "open"|"closed"|"merged"; author: String
    base: String; head: String; url: URI
    created_at: Time; updated_at: Time
```

---

## 35. Browser

Web browser automation (Playwright/Puppeteer).

**Configuration:**
```lucky
tool Browser(
    headless = true,
    timeout = 30s,
    viewport = { width: 1280, height: 720 },
)
```

**Navigation:** `navigate(url)`, `reload()`, `back()`, `forward()`, `close()`, `title()`, `url()`, `content()`

**Interaction:** `click(sel)`, `double_click(sel)`, `hover(sel)`, `type(sel, text)`, `press(key)`, `select(sel, val)`, `check(sel)`, `uncheck(sel)`, `upload_file(sel, path)`, `scroll_to(sel)`, `execute_js(code)`, `wait_for_selector(sel, dur?)`, `wait_for_navigation(dur?)`

**Extraction:** `extract(sel)`, `extract_all(sel)`, `extract_attribute(sel, attr)`, `extract_table(sel)`, `extract_links()`, `extract_markdown(sel?)`, `evaluate(js)`

**Media:** `screenshot(path?, sel?)`, `pdf(path?, opts?)`

**Auth:** `login(url, user, pass, user_sel?, pass_sel?)`, `login_oauth(url, provider, timeout?)`

---

## 36. Shell

Controlled command-line execution.

**Configuration:**
```lucky
tool Shell(
    workdir = "./project",
    allowed_commands = ["ls", "cat", "grep", "find", "cargo", "npm", "python", "node", "go", "make", "git"],
    denied_patterns = ["rm -rf", "sudo", "chmod 777", "curl * | sh"],
    allow_pipes = true,
    allow_redirects = false,
    timeout = 5m,
)
```

| Method | Signature | Description |
|---|---|---|
| `exec` | `(String, ExecOptions?) -> Result<ShellOutput>` | Execute |
| `exec_batch` | `(List<String>, ExecOptions?) -> Result<List<ShellOutput>>` | Batch |
| `which` | `(String) -> Result<Path>` | Locate executable |
| `cd` | `(Path) -> ()` | Change dir |
| `pwd` | `() -> Path` | Current dir |
| `env` | `() -> Map<String,String>` | Environment |
| `set_env` | `(String, String) -> ()` | Set env var |
| `platform` | `() -> String` | "windows"/"linux"/"macos" |

**ExecOptions:** `{ workdir?, env?, timeout?, capture_stdout?, capture_stderr?, stdin?, check? }`
**ShellOutput:** `{ command, stdout, stderr, exit_code, success, duration }`

---

## 37. HTTP

HTTP client with retry, auth, and streaming.

**Configuration:**
```lucky
tool HTTP(
    base_url = "https://api.example.com",
    default_headers = { "Authorization": "Bearer \{token}" },
    timeout = 30s,
    retry = 3,
    retry_backoff = exponential,
)
```

**Standard methods:** `get(url, opts?)`, `post(url, body?, opts?)`, `put(url, body?, opts?)`, `patch(url, body?, opts?)`, `delete(url, opts?)`, `head(url, opts?)`, `options(url, opts?)`, `request(method, url, body?, opts?)`

**Convenience:** `get_json(url, opts?)`, `post_json(url, body?, opts?)`, `download(url, path, opts?)`, `upload(url, path, opts?)`, `stream(url, method?, opts?)`

**WebSocket:** `ws_connect(url)`

**RequestOptions:** `{ headers?, query?, timeout?, retry?, follow_redirects?, auth? }`
**AuthMethod:** `Basic { username, password } | Bearer { token } | ApiKey { key, header }`
**HttpResponse:** `{ status, status_text, headers, body, url, elapsed, is_success, is_client_error, is_server_error, json(), text(), cookies() }`
**WebSocket:** `send(s), send_json(v), recv(), recv_json(), close(), stream()`

---

## 38. Database

Database access with connection pooling.

**Configuration:**
```lucky
tool Database(
    url = "postgresql://localhost:5432/mydb",
    pool_size = 10,
    timeout = 30s,
)
```

| Method | Signature | Description |
|---|---|---|
| `query` | `(String, List<Any>?) -> Result<List<Row>>` | Query |
| `query_one` | `(String, List<Any>?) -> Result<Row?>` | Single row |
| `execute` | `(String, List<Any>?) -> Result<Int>` | Execute DML |
| `execute_batch` | `(List<(String,List<Any>?)>) -> Result<List<Int>>` | Batch |
| `transaction` | `(fn(Database)->Result<T>) -> Result<T>` | Transaction |
| `tables` | `() -> Result<List<String>>` | List tables |
| `columns` | `(String) -> Result<List<ColumnDef>>` | Describe table |
| `close` | `() -> ()` | Close pool |

**Row:** `get(col)`, `get_typed::<T>(col)`, `get_int(col)`, `get_float(col)`, `get_string(col)`, `get_bool(col)`, `get_time(col)`, `columns()`, `to_map()`

---

## 39. Search

Web search and information retrieval.

**Configuration:**
```lucky
tool Search(
    provider = "tavily",
    api_key = Secret.new(env.get("SEARCH_API_KEY")),
    max_results = 10,
    search_depth = "advanced",
)
```

| Method | Signature | Description |
|---|---|---|
| `search` | `(String, SearchOptions?) -> Result<SearchResults>` | Web search |
| `search_news` | `(String, SearchOptions?) -> Result<SearchResults>` | News search |
| `search_images` | `(String, SearchOptions?) -> Result<SearchResults>` | Image search |
| `search_scholar` | `(String, SearchOptions?) -> Result<SearchResults>` | Academic search |

**SearchOptions:** `{ max_results?, include_answer?, search_depth?, time_range?, domain_filter? }`
**SearchResults:** `{ query, answer?, results[], total_results, search_time }`
**SearchResult:** `{ title, url: URI, content, raw_content?, score, published_at? }`

---

# Part V -- Standard Agents

Pre-built agent definitions for common AI orchestration patterns.

---

## 40. Planner

```lucky
agent Planner
    model Claude
    prompt PlannerPrompt
    task Decompose
        input goal: String; constraints: List<String>; context: Map<String,Any>?
        output plan: Plan
    task Replan
        input original_plan: Plan; feedback: String; current_state: Map<String,Any>
        output updated_plan: Plan
    task Prioritize
        input tasks: List<Task>; criteria: List<String>
        output ordered_tasks: List<Task>
```

---

## 41. Researcher

```lucky
agent Researcher
    model Claude
    tools [Search, Browser, HTTP]
    task Investigate
        input topic: String; depth: "quick"|"standard"|"deep"; sources: List<String>?
        output report: Document; citations: List<Citation>
    task FactCheck
        input claim: String; sources: List<URI>?
        output verdict: FactCheckResult
    task LiteratureReview
        input topic: String; papers: List<URI>?
        output review: Document
```

---

## 42. Coder

```lucky
agent Coder
    model Claude
    tools [Filesystem, Git, Shell]
    task Generate
        input specification: String; language: String; framework: String?; existing_code: String?
        output code: String; explanation: String
    task Refactor
        input code: String; goal: String; constraints: List<String>?
        output refactored_code: String; changes_summary: String
    task FixBug
        input code: String; bug_description: String; error_message: String?; stack_trace: String?
        output fixed_code: String; root_cause: String; fix_explanation: String
    task ReviewOwnCode
        input code: String; requirements: String
        output review: CodeReview
```

---

## 43. Reviewer

```lucky
agent Reviewer
    model Claude
    tools [Git, Filesystem]
    task ReviewCode
        input code: String|List<FileDiff>; focus_areas: List<String>?
        output review: CodeReview
    task ReviewDocument
        input document: String; document_type: String; criteria: List<String>?
        output review: DocumentReview
    task ReviewArchitecture
        input architecture_doc: String; constraints: Map<String,Any>?
        output review: ArchitectureReview
```

**CodeReview:** `{ summary, overall_assessment: "approve"|"approve_with_comments"|"request_changes", findings[], suggestions[] }`
**Finding:** `{ severity: "info"|"low"|"medium"|"high"|"critical", category, file?, line?, description, recommendation, code_snippet? }`

---

## 44. Tester

```lucky
agent Tester
    model Claude
    tools [Filesystem, Shell, Git]
    task GenerateTests
        input code: String; language: String; test_framework: String?; coverage_target: Float?
        output tests: String; test_plan: String
    task RunTests
        input command: String; timeout: Duration?
        output results: TestResults
    task AnalyzeFailures
        input test_results: TestResults; source_code: String?
        output analysis: FailureAnalysis
    task GenerateRegressionTests
        input bug_description: String; fixed_code: String; original_code: String?
        output regression_tests: String
```

**TestResults:** `{ total, passed, failed, skipped, duration, failures[], coverage? }`
**TestFailure:** `{ name, message, file?, line?, stack_trace? }`

---

## 45. Architect

```lucky
agent Architect
    model Claude
    tools [Search, Browser]
    task DesignSystem
        input requirements: String; constraints: Map<String,Any>?; existing_architecture: String?
        output architecture: ArchitectureDocument; tradeoffs: List<Tradeoff>
    task EvaluateTechnology
        input options: List<String>; criteria: Map<String,Float>
        output recommendation: TechnologyRecommendation
    task DesignAPI
        input requirements: String; style: "rest"|"graphql"|"grpc"; existing_schema: String?
        output api_spec: String
```

---

## 46. SecurityAuditor

```lucky
agent SecurityAuditor
    model Claude
    tools [Filesystem, Git, Shell]
    task AuditCode
        input code: String|List<FileDiff>; language: String
        output report: SecurityReport
    task AuditDependencies
        input dependency_file: String
        output report: DependencyReport
    task PenetrationTest
        input target: URI; scope: List<String>?
        output report: PenetrationTestReport
    task ThreatModel
        input system_description: String; data_flow_diagram: String?
        output threat_model: ThreatModel
```

---

## 47. TechnicalWriter

```lucky
agent TechnicalWriter
    model Claude
    tools [Filesystem, Git]
    task WriteDocs
        input source: String|URI; doc_type: "api"|"guide"|"readme"|"changelog"|"architecture"|"tutorial"
        input audience: "developer"|"user"|"executive"; template: String?
        output document: String
    task ImproveDocs
        input existing_docs: String; feedback: String?
        output improved_docs: String; changes: List<String>
    task GenerateChangelog
        input commits: List<Commit>?; version: Version; previous_version: Version?
        output changelog: String
```

---

# Part VI -- Standard AI Module

---

## 48. ai Module

```lucky
import ai
```

| Function | Signature | Description |
|---|---|---|
| `ai.ask` | `(String, AIOptions?) -> Probabilistic<String>` | General QA |
| `ai.chat` | `(List<Message>, AIOptions?) -> Message` | Multi-turn |
| `ai.summarize` | `(String, SummarizeOptions?) -> String` | Summarize |
| `ai.translate` | `(String, String, AIOptions?) -> String` | Translate to lang |
| `ai.extract_keywords` | `(String, Int?) -> List<String>` | Keywords |
| `ai.sentiment` | `(String) -> SentimentResult` | Sentiment |
| `ai.classify` | `(String, List<String>, AIOptions?) -> ClassificationResult` | Classify |
| `ai.extract_entities` | `(String, List<String>?) -> List<Entity>` | NER |
| `ai.generate_code` | `(String, String, CodeGenOptions?) -> String` | Code gen |
| `ai.explain_code` | `(String) -> String` | Explain code |
| `ai.fix_code` | `(String, String?) -> String` | Fix code |
| `ai.review_code` | `(String, ReviewOptions?) -> CodeReview` | Review code |
| `ai.generate_tests` | `(String, String?) -> String` | Test gen |
| `ai.generate_image` | `(String, ImageOptions?) -> Image` | Text-to-image |
| `ai.analyze_image` | `(Image, String?) -> String` | Image desc |
| `ai.transcribe` | `(Bytes|Path, TranscribeOptions?) -> String` | Speech-to-text |
| `ai.synthesize_speech` | `(String, SpeechOptions?) -> Bytes` | Text-to-speech |
| `ai.compare` | `(String, String, CompareOptions?) -> ComparisonResult` | Compare |
| `ai.paraphrase` | `(String, ParaphraseOptions?) -> String` | Rewrite |
| `ai.brainstorm` | `(String, Int?) -> List<String>` | Ideas |
| `ai.outline` | `(String, Int?) -> String` | Outline |
| `ai.answer_with_rag` | `(String, Knowledge, Int?) -> Answer` | RAG answer |

**AIOptions:** `{ model?, temperature?, max_tokens?, min_confidence?, system_prompt?, examples? }`
**SentimentResult:** `{ label: "positive"|"negative"|"neutral"|"mixed", score: Float, confidence: Float }`
**ClassificationResult:** `{ category: String, confidence: Float, alternatives: List<(String,Float)> }`
**Entity:** `{ text: String, type: String, start: Int, end: Int, confidence: Float }`

---

## 49. text, embeddings, rag Modules

### text Module
```lucky
import text
```

| Function | Signature | Description |
|---|---|---|
| `text.tokenize` | `(String) -> List<String>` | Word tokenization |
| `text.sentence_tokenize` | `(String) -> List<String>` | Sentence split |
| `text.word_count` | `(String) -> Int` | Word count |
| `text.ngrams` | `(String, Int) -> List<String>` | N-grams |
| `text.cosine_similarity` | `(Map<String,Float>, Map<String,Float>) -> Float` | |
| `text.jaccard_similarity` | `(Set<String>, Set<String>) -> Float` | |
| `text.levenshtein` | `(String, String) -> Int` | Edit distance |
| `text.jaro_winkler` | `(String, String) -> Float` | Similarity |
| `text.slugify` | `(String) -> String` | URL slug |
| `text.truncate` | `(String, Int, String?) -> String` | Truncate |
| `text.wrap` | `(String, Int) -> String` | Word wrap |
| `text.strip_ansi` | `(String) -> String` | ANSI strip |
| `text.markdown_to_html` | `(String) -> String` | MD to HTML |
| `text.html_to_text` | `(String) -> String` | HTML strip |
| `text.extract_urls` | `(String) -> List<URI>` | Extract URLs |
| `text.extract_emails` | `(String) -> List<String>` | Extract emails |

### embeddings Module
```lucky
import embeddings
```

| Function | Signature | Description |
|---|---|---|
| `embeddings.embed` | `(String, EmbeddingOptions?) -> Embedding` | Generate |
| `embeddings.embed_batch` | `(List<String>, ...) -> List<Embedding>` | Batch |
| `embeddings.cosine_similarity` | `(Embedding, Embedding) -> Float` | |
| `embeddings.euclidean_distance` | `(Embedding, Embedding) -> Float` | |
| `embeddings.dot_product` | `(Embedding, Embedding) -> Float` | |
| `embeddings.normalize` | `(Embedding) -> Embedding` | L2 normalize |
| `embeddings.dimensions` | `(Embedding) -> Int` | |
| `embeddings.to_list` | `(Embedding) -> List<Float>` | |
| `embeddings.from_list` | `(List<Float>) -> Embedding` | |

### rag Module
```lucky
import rag
```

| Function | Signature | Description |
|---|---|---|
| `rag.query` | `(String, Knowledge, RagOptions?) -> Answer` | RAG query |
| `rag.build_index` | `(List<Document>, IndexOptions?) -> Index` | Build index |
| `rag.search` | `(String, Index, Int?) -> List<ScoredDocument>` | Vector search |
| `rag.rerank` | `(String, List<Document>, ...) -> List<ScoredDocument>` | Rerank |
| `rag.hybrid_search` | `(String, Index, Float?) -> List<ScoredDocument>` | Hybrid |
| `rag.chunk_document` | `(String, ChunkOptions?) -> List<String>` | Chunk |
| `rag.merge_chunks` | `(List<ScoredDocument>, ...) -> String` | Merge |

---

# Part VII -- Utility Modules

---

## 50. time Module

```lucky
import time
```

| Function | Signature | Description |
|---|---|---|
| `time.now` | `() -> Time` | Current UTC |
| `time.now_local` | `() -> Time` | Current local |
| `time.epoch` | `() -> Time` | Unix epoch |
| `time.sleep` | `(Duration) -> ()` | Block |
| `time.measure` | `(fn()->T) -> (T, Duration)` | Measure |
| `time.deadline` | `(Duration) -> Time` | now + duration |
| `time.since` | `(Time) -> Duration` | Duration since |
| `time.until` | `(Time) -> Duration` | Duration until |
| `time.schedule` | `(String, fn()->()) -> ScheduledTask` | Cron |
| `time.every` | `(Duration, fn()->()) -> ScheduledTask` | Interval |
| `time.at` | `(Time, fn()->()) -> ScheduledTask` | One-time |
| `time.timer` | `(Duration, fn()->()) -> Timer` | Fire/forget |
| `time.ticker` | `(Duration, fn()->()) -> Ticker` | Periodic |
| `time.timezone` | `() -> String` | System tz |
| `time.timezones` | `() -> List<String>` | IANA tz names |

---

## 51. math Module

```lucky
import math
```

**Constants:** `pi`, `e`, `tau`, `phi`, `sqrt2`, `inf`, `nan`

**Number:** `abs`, `sign`, `min`, `max`, `clamp`, `round`, `floor`, `ceil`, `trunc`, `fract`, `sqrt`, `cbrt`, `pow`, `exp`, `exp2`, `ln`, `log2`, `log10`, `log`, `hypot`, `gcd`, `lcm`, `is_prime`, `factor`, `factorial`, `binomial`, `fibonacci`

**Trigonometry:** `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `sinh`, `cosh`, `tanh`, `degrees`, `radians`

**Statistics:** `mean`, `median`, `mode`, `variance`, `stdev`, `percentile`, `quartiles`, `iqr`, `skewness`, `kurtosis`, `covariance`, `correlation`, `linear_regression`

---

## 52. random, collections, json, yaml, log, env Modules

### random
```lucky
import random
```
`int(lo, hi)`, `float()`, `bool()`, `choice(list)`, `choices(list, n, weights?)`, `sample(list, n)`, `shuffle(list)`, `uniform(lo, hi)`, `gauss(mu, sigma)`, `seed(n)`, `uuid()`

### collections
```lucky
import collections
```
`is_empty(c)`, `len(c)`, `to_list(it)`, `to_set(it)`, `to_stream(it)`, `zip(a,b)`, `enumerate(list)`, `reverse(list)`, `flatten(nested)`, `transpose(matrix)`, `cartesian_product(a,b)`, `chunks(list,n)`, `windows(list,n)`, `rotate_left(list,n)`, `rotate_right(list,n)`, `pad_left(list,n,v)`, `pad_right(list,n,v)`

### json
```lucky
import json
```
`encode(v, opts?) -> Result<String>`, `encode_pretty(v)`, `decode(s) -> Result<Any>`, `decode_typed::<T>(s)`, `validate(s, schema?)`

### yaml
```lucky
import yaml
```
`encode(v)`, `decode(s)`, `decode_typed::<T>(s)`, `decode_all(s)`

### log
```lucky
import log
```
`trace(msg, data?)`, `debug(msg, data?)`, `info(msg, data?)`, `warn(msg, data?)`, `error(msg, data?)`, `fatal(msg, data?)`, `level(lvl)`, `format("json"|"text")`, `with_context(data) -> Logger`

### env
```lucky
import env
```
`get(key)`, `get_required(key)`, `get_typed::<T>(key)`, `get_or(key, def)`, `set(key, val)`, `remove(key)`, `all()`, `args()`, `home()`, `temp()`, `cwd()`, `hostname()`, `pid()`

---

## 53. Runtime API

```lucky
import runtime
```

| Function | Signature | Description |
|---|---|---|
| `runtime.version` | `() -> Version` | Runtime version |
| `runtime.config` | `() -> Map<String,Any>` | Current config |
| `runtime.run_id` | `() -> UUID` | Current run ID |
| `runtime.status` | `() -> RunStatus` | Current status |
| `runtime.progress` | `() -> Float` | 0.0-1.0 |
| `runtime.current_node` | `() -> String?` | Active node |
| `runtime.cancel` | `() -> ()` | Cancel run |
| `runtime.pause` | `() -> ()` | Pause execution |
| `runtime.resume` | `() -> ()` | Resume execution |
| `runtime.checkpoint` | `() -> UUID` | Manual checkpoint |
| `runtime.restore` | `(UUID) -> ()` | Restore checkpoint |
| `runtime.cost` | `() -> CostReport` | Cost so far |
| `runtime.nodes` | `() -> List<NodeStatus>` | All node statuses |
| `runtime.on_event` | `(fn(RunEvent)->()) -> ()` | Register callback |
| `runtime.shutdown` | `() -> ()` | Graceful shutdown |

---

## Appendix A -- Complete Type Hierarchy

```
Any
 +-- Bool, Int, Float, Decimal
 +-- String, Char, Bytes
 +-- Time, Duration, UUID, URI, Version, Path
 +-- null, unknown
 +-- Error
 +-- Result<T,E>
 +-- Secret<T>
 +-- Probabilistic<T>
 +-- List<T>, Set<T>, Map<K,V>
 +-- Queue<T>, Stack<T>
 +-- Graph<N,E>, Tree<T>
 +-- Stream<T>, Range
 +-- Channel<T>, Promise<T>
 +-- Model, Prompt, Agent, Task<I,O>, Workflow<I,O>, Goal
 +-- Memory, Knowledge, Context, Tool
 +-- Capability, Approval, Reasoning
 +-- Embedding, Artifact<T>, Plan, Observation
```

---

## Appendix B -- Error Codes

| Code | Name | Description |
|---|---|---|
| 1 | INTERNAL | Internal runtime error |
| 2 | NOT_FOUND | Resource not found |
| 3 | PERMISSION_DENIED | Capability security violation |
| 4 | INVALID_ARGUMENT | Bad input |
| 5 | TIMEOUT | Operation timed out |
| 6 | CANCELLED | Operation cancelled |
| 7 | ALREADY_EXISTS | Resource already exists |
| 8 | UNAVAILABLE | Service unavailable (transient) |
| 9 | RESOURCE_EXHAUSTED | Memory/budget/rate limit |
| 10 | DEADLINE_EXCEEDED | Total time budget exceeded |
| 11 | PRECONDITION_FAILED | Precondition not met |
| 12 | ABORTED | Concurrency conflict |
| 13 | OUT_OF_RANGE | Value out of range |
| 14 | UNIMPLEMENTED | Not implemented |
| 15 | DATA_LOSS | Unrecoverable data loss |
| 20 | TYPE_ERROR | Type mismatch |
| 21 | PARSE_ERROR | Parse/syntax error |
| 22 | VALIDATION_ERROR | Schema validation failure |
| 30 | MODEL_ERROR | LLM model error |
| 31 | MODEL_REFUSAL | LLM safety refusal |
| 40 | TOOL_ERROR | Tool execution failure |
| 41 | SANDBOX_ERROR | Sandbox violation |
| 50 | HUMAN_REJECTED | Human rejected approval |
| 51 | HUMAN_TIMEOUT | Human approval timed out |
| 60 | BUDGET_EXCEEDED | Cost budget exceeded |
| 61 | RATE_LIMITED | API rate limit hit |

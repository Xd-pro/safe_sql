Disclaimer: This is my first Rust project ever, I'm open to constructive feedback about how the code is structured etc.
# safe_sql
Code generation tool for PHP making PDO queries more type-safe and easier to autocomplete. Works with any database supported by PDO (incl. MySql, SQLite, PostgreSQL)

Generates zero-dependency code, so it can be used in [Pocketmine plugins](https://github.com/pmmp/PocketMine-MP), where composer dependencies can't easily be used. If you are using it in a PocketMine plugin, PLEASE use async tasks in a new thread pool to avoid lag!
# Installation
Download the binary from the releases section. Keep base.php in the working directory.

It is responsible for the base class and namespace, so make sure to add a correct namespace after the <?php tag in the file. 
By default, the `out.php` file will be written when you run the executable, but passing in a command line argument can override this: `./safe_sql src/name/project/database/Transaction.php`
# Basic usage
Create a directory called queries in the working directory. Inside, you will put SQL files with special syntax for PHP types:

`books.sql`
```sql
--#blurb_by_name
SELECT @Blurb: string FROM Books WHERE BookName = $BookName: string;
--#create_table
CREATE TABLE Books (Id INT PRIMARY KEY, BookName TEXT, Blurb TEXT);
--#insert
INSERT INTO Books (Id, BookName, Blurb) VALUES ($id: int, $bookname: string, $blurb: string);
```
Be careful not to forget semicolons!

The comments prefixed with a hash are the names of queries. As you can see, there is some very not normal SQL syntax in the queries.
## Variables
Variables can be created in queries using the dollar sign:
```sql
INSERT INTO Books (Id, BookName, Blurb) VALUES ($id: int, $bookname: string, $blurb: string);
```
The type after the colon is a PHP type that will be used when generating the functions for your queries. When you run the executable, this query is transformed into PHP:
```php
class Transaction extends TransactionBase
{
    /** @return int */ public function books_insert(int $id, string $bookname, string $blurb,)
    {
        $statement = $this->db->prepare("INSERT INTO Books (Id, BookName, Blurb) VALUES (?, ?, ?) ");
        $statement->execute([$id, $bookname, $blurb,]);
        return $this->db->lastInsertId();
    }
}
```
As you can see, the function returns `lastInsertId`. This can be safely ignored if you aren't using auto-incremented IDs. To use this query in your codebase:
```php
$t = new Transaction($pdo);
try {
  $t->books_insert(1, "The GFO", "The giant friendly orge is an award winning book set in...");
  $t->commit();
} catch (Exception $e) {
  $t->rollBack();
}
```
## Return values
Return values allow you to actually query your data. For example:
```sql
--#blurb_by_name
SELECT @Blurb: string FROM Books WHERE BookName = $BookName: string;
```
The @ sign declares a return value. 
The `Blurb` before the colon is the name of the field in your database, and the type after it is another PHP type. When this code is turned into normal SQL, the @ sign and the type are dropped, leaving only the name of the field, resulting in valid SQL.
Here is the PHP output: 
```php
    /** @return books_blurb_by_name[]|Generator */ public function books_blurb_by_name(string $BookName,)
    {
        $statement = $this->db->prepare("SELECT Blurb FROM Books WHERE BookName = ? ");
        $statement->execute([$BookName,]);
        while ($res = $statement->fetch(PDO::FETCH_NUM)) {
            yield new books_blurb_by_name(...$res);
        }
    }
```
Don't be frightened by the generator, it's not as bad as you think it is. The real magic of code generation is the autocomplete you can get on the return type. A new class is also generated, and it looks like this:
```php
class books_blurb_by_name
{
    public function __construct(public string $Blurb,)
    {
    }
}
```
It has one member, which is the returned value from the query. You can return as many values as you want from a query, and it's easy to do so. Just add more `@Whatever: string`! Anyway, using this in your code is simple:
```php
$t = new Transaction(...);
try {
    $result = $t->books_blurb_by_name("The GFO");
    $t->commit();
    foreach ($result as $res) {
        echo $res->Blurb . "\n";
    }
} catch (Exception $e) {
    $t->rollBack();
}
```
# Async (PocketMine-MP)
First, bootstrap the thread pool used for async in onEnable:
```php
class MyPlugin extends PluginBase {

    public DatabasePool $db;

    public function onEnable(): void {
        $this->db = SafeSql::bootstrapPocketmine($this, "Your PDO connection string");
    }

}
```
To run a query we need to create an `AsyncTransaction`:
```php
class AT_BooksBlurbByName extends AsyncTransaction {

    public function __construct(private string $name) {}

    public function run() {
        $result = $t->books_blurb_by_name("The GFO");
        $t->commit(); // if you don't commit or rollBack, a warning is produced in the console and the data is rolled back! This is to ensure unintentional updates don't happen.
        foreach ($result as $res) {
            return $res;
        }
    }

}
```
And then pass it into `DatabasePool::run`:
```php
$this->db->run(new AT_BooksBlurbByName, function(books_blurb_by_name|Exception $data) {
    if ($data instanceof books_blurb_by_name) {
        var_dump($data->Blurb);
    }
});
```
# Planned
- Remove sometimes unnecessary call to `PDO::lastInsertId`
- Support multiple databases at once (similar to how [libAsynql](https://github.com/poggit/libAsynql/) does)
- Better syntax error reporting in SQL files
- Migration system

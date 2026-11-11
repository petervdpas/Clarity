// The data formats of code end to end on the C++ runtime
// (conformance/formats-runtime.json): read through Clarity.Formats, decoded
// by nlohmann into records as the generator writes them, written back.

#include <filesystem>
#include <fstream>
#include <sstream>

#include "check.hpp"
#include "clarity/sql.hpp"
#include "conformance.hpp"
#include "formats/file_source.hpp"
#include "formats/formats.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

struct Line {
    int64_t id = 0;
    std::string product;
    int64_t quantity = 0;
};

struct Order {
    int64_t id = 0;
    std::string number;
    Decimal total;
    std::optional<Date> placed;
    std::vector<Line> lines;
};

void to_json(json& j, const Line& r) { j = json{{"id", r.id}, {"product", r.product}, {"quantity", r.quantity}}; }
void from_json(const json& j, Line& r) {
    j.at("id").get_to(r.id);
    j.at("product").get_to(r.product);
    j.at("quantity").get_to(r.quantity);
}
void to_json(json& j, const Order& r) { j = json{{"id", r.id}, {"number", r.number}, {"total", r.total}, {"placed", r.placed}, {"lines", r.lines}}; }
void from_json(const json& j, Order& r) {
    j.at("id").get_to(r.id);
    j.at("number").get_to(r.number);
    j.at("total").get_to(r.total);
    j.at("placed").get_to(r.placed);
    j.at("lines").get_to(r.lines);
}

}  // namespace

TEST(formats_read_decode_and_write_as_every_stack_does) {
    for (const auto& c : conformance("formats-runtime.json")["cases"]) {
        std::string kind = c["kind"].dump();
        std::string options = c.contains("options") ? c["options"].dump() : "";
        std::string input = c["input"].get<std::string>();
        bool many = c["kind"]["kind"] == "records";
        std::string format = c.value("format", "json");
        try {
            std::string written = many ? formats_write(format, formats_read<std::vector<Order>>(format, input, options, kind), options, kind)
                                       : formats_write(format, formats_read<Order>(format, input, options, kind), options, kind);
            CHECK(c.contains("written"));
            if (c.contains("written")) CHECK_EQ(written, c["written"].get<std::string>());
        } catch (const ValueError& e) {
            CHECK(c.contains("error"));
            if (c.contains("error")) CHECK_EQ(std::string(e.what()), c["error"].get<std::string>());
        }
    }
}

// A data source that is a file in a format (docs/FORMATS-PLAN.md): a CSV
// read into a table its entity finds, saves and deletes in, each change
// written back to the file with its own column order and the columns no
// entity maps kept.
namespace {

struct Login {
    int64_t id = 0;
    std::string site;
    int64_t visits = 0;
    bool paid = false;
    static constexpr const char* table = "login";
    static constexpr const char* source = "logins";
    static const std::vector<std::string>& columns() {
        static const std::vector<std::string> names{"Site", "visits", "paid"};
        return names;
    }
    std::vector<SqlValue> values() const { return {sql::text(site), sql::integer(visits), sql::boolean(paid)}; }
    static Login from_values(int64_t id, const Row& v) { return {id, sql::get_text("site", v[0]), sql::get_integer("visits", v[1]), sql::get_boolean("paid", v[2])}; }
};

FileSource login_source(const std::string& path) {
    return {"logins", "csv", path, R"({"separator":";"})", "login", "id",
            R"([{"name":"Site","kind":"text"},{"name":"visits","kind":"integer"},{"name":"paid","kind":"bool"}])", false};
}

void write_text(const std::filesystem::path& path, const std::string& text) {
    std::ofstream out(path, std::ios::binary | std::ios::trunc);
    out << text;
}

std::string read_text(const std::filesystem::path& path) {
    std::ifstream in(path, std::ios::binary);
    std::ostringstream text;
    text << in.rdbuf();
    return text.str();
}

}  // namespace

TEST(a_csv_file_is_a_data_source_written_back_after_each_change) {
    namespace fs = std::filesystem;
    fs::path folder = fs::temp_directory_path() / "clarity-file-source-cpp";
    fs::remove_all(folder);
    fs::create_directories(folder);
    fs::path file = folder / "logins.csv";
    write_text(file, "Site;Note;visits;paid\r\nexample.com;keep me;3;true\r\nshop.nl;;1;false\r\n");
    {
        Db db(std::vector<Source>{});
        db.add_file(login_source(file.string()));

        auto all = db.all<Login>();
        REQUIRE(all.size() == 2);
        CHECK_EQ(all[0].id, int64_t(1));
        CHECK_EQ(all[0].site, std::string("example.com"));
        CHECK_EQ(all[0].visits, int64_t(3));
        CHECK(all[0].paid);
        CHECK_EQ(all[1].id, int64_t(2));
        CHECK_EQ(all[1].site, std::string("shop.nl"));
        CHECK_EQ(all[1].visits, int64_t(1));
        CHECK(!all[1].paid);

        all[0].visits = 4;
        db.save(all[0]);
        Login added{0, "a;b", 0, true};
        db.save(added);
        db.remove(all[1]);
        CHECK_EQ(read_text(file), std::string("Site;Note;visits;paid\r\nexample.com;keep me;4;true\r\n\"a;b\";;0;true\r\n"));
        // The new file replaced the old one.
        CHECK(!fs::exists(folder / "logins.csv.new"));
    }

    // A column the entity maps that the file lacks stops the program as it opens.
    write_text(file, "Site;paid\r\n");
    Db db(std::vector<Source>{});
    std::string error;
    try {
        db.add_file(login_source(file.string()));
    } catch (const StorageError& e) {
        error = e.what();
    }
    CHECK(error.find("data source logins: the CSV has no column 'visits'") != std::string::npos);

    // A file not there yet starts empty, and the first save makes it and its folder.
    fs::path fresh = folder / "fresh" / "logins.csv";
    {
        Db made(std::vector<Source>{});
        made.add_file(login_source(fresh.string()));
        CHECK(made.all<Login>().empty());
        Login first{0, "new.nl", 0, true};
        made.save(first);
        CHECK_EQ(read_text(fresh), std::string("Site;visits;paid\r\nnew.nl;0;true\r\n"));
        // A readonly source needs its file.
        FileSource needed = login_source((folder / "missing.csv").string());
        needed.readonly = true;
        std::string missing;
        try {
            made.add_file(needed);
        } catch (const StorageError& e) {
            missing = e.what();
        }
        CHECK(missing.find("data source logins: cannot read") != std::string::npos);
    }
    fs::remove_all(folder);
}

// A file source whose mapping holds records inside its records (`nested`):
// each entity in its own table, a line linked to its order by the order's key.
namespace {

struct FeedOrder {
    int64_t id = 0;
    std::string number;
    static constexpr const char* table = "order";
    static constexpr const char* source = "feed";
    static const std::vector<std::string>& columns() {
        static const std::vector<std::string> names{"number"};
        return names;
    }
    std::vector<SqlValue> values() const { return {sql::text(number)}; }
    static FeedOrder from_values(int64_t id, const Row& v) { return {id, sql::get_text("number", v[0])}; }
};

struct FeedLine {
    int64_t id = 0;
    int64_t order = 0;
    std::string product;
    static constexpr const char* table = "line";
    static constexpr const char* source = "feed";
    static const std::vector<std::string>& columns() {
        static const std::vector<std::string> names{"order", "product"};
        return names;
    }
    std::vector<SqlValue> values() const { return {sql::integer(order), sql::text(product)}; }
    static FeedLine from_values(int64_t id, const Row& v) { return {id, sql::get_integer("order", v[0]), sql::get_text("product", v[1])}; }
};

}  // namespace

TEST(a_file_source_holds_the_records_inside_its_records_in_their_own_table) {
    namespace fs = std::filesystem;
    fs::path folder = fs::temp_directory_path() / "clarity-file-source-nested-cpp";
    fs::remove_all(folder);
    fs::create_directories(folder);
    fs::path file = folder / "orders.xml";
    write_text(file,
               "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<shop>\n  <orders>\n    <order number=\"A-1\"><lines><line product=\"Tea\"/><line product=\"Coffee\"/></lines></order>\n"
               "    <order number=\"A-2\"><lines><line product=\"Milk\"/></lines></order>\n  </orders>\n</shop>\n");
    {
        Db db(std::vector<Source>{});
        db.add_file({"feed", "xml", file.string(),
                     R"({"at":"shop/orders/order","fields":{"number":"@number"},"nested":{"lines":{"at":"lines/line","fields":{"product":"@product"},"parent":"order"}}})",
                     "order", "id",
                     R"([{"name":"number","kind":"text"},{"name":"lines","kind":"records","entity":"Line","fields":[{"name":"product","kind":"text"}],"base":{},"table":"line","key":"id"}])",
                     false});
        auto orders = db.all<FeedOrder>();
        REQUIRE(orders.size() == 2);
        CHECK_EQ(orders[0].id, int64_t(1));
        CHECK_EQ(orders[0].number, std::string("A-1"));
        CHECK_EQ(orders[1].id, int64_t(2));
        CHECK_EQ(orders[1].number, std::string("A-2"));
        auto lines = db.all<FeedLine>();
        REQUIRE(lines.size() == 3);
        CHECK_EQ(lines[0].order, int64_t(1));
        CHECK_EQ(lines[0].product, std::string("Tea"));
        CHECK_EQ(lines[1].order, int64_t(1));
        CHECK_EQ(lines[1].product, std::string("Coffee"));
        CHECK_EQ(lines[2].id, int64_t(3));
        CHECK_EQ(lines[2].order, int64_t(2));
        CHECK_EQ(lines[2].product, std::string("Milk"));

        FeedLine added{0, 2, "Juice"};
        db.save(added);
        db.remove(lines[0]);
        CHECK_EQ(read_text(file), std::string("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<shop>\n  <orders>\n    <order number=\"A-1\"><lines><line product=\"Coffee\"/></lines></order>\n"
                                              "    <order number=\"A-2\"><lines><line product=\"Milk\"/><line product=\"Juice\"/></lines></order>\n  </orders>\n</shop>\n"));
    }
    fs::remove_all(folder);
}

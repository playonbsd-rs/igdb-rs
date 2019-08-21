use std::collections::BTreeMap;
use std::str::FromStr;
use std::string::ToString;
use surf::middleware::HttpClient;
use url::Url;

const ALL_FIELDS: &'static str = "*";
const HEADER_KEY_NAME: &'static str = "user-key";

struct Filter {
    key: String,
    symbol: String,
    value: String,
}

pub struct RequestBuilder {
    fields: Vec<String>,
    filters: Vec<Filter>,
    sort: (String, String),
    limit: usize,
    search: String,
}

pub enum OrderBy {
    Descending,
    Ascending,
}

impl ToString for OrderBy {
    fn to_string(&self) -> String {
        match self {
            OrderBy::Ascending => "asc",
            OrderBy::Descending => "desc",
        }
        .into()
    }
}

pub enum Equality {
    Lower,
    Greater,
    Equal,
}

impl ToString for Equality {
    fn to_string(&self) -> String {
        match self {
            Equality::Equal => "=",
            Equality::Greater => ">",
            Equality::Lower => "<",
        }
        .into()
    }
}

impl RequestBuilder {
    pub fn new() -> RequestBuilder {
        RequestBuilder {
            fields: Vec::new(),
            filters: vec![],
            sort: (String::new(), String::new()),
            limit: 50,
            search: String::new(),
        }
    }
    pub fn all_fields(&mut self) -> &mut Self {
        self.fields.clear();
        self.fields.push(ALL_FIELDS.into());
        self
    }

    pub fn add_field<S: Into<String>>(&mut self, field: S) -> &mut Self {
        self.fields.push(field.into());
        self
    }

    pub fn add_fields<I, T>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let str_fields: Vec<String> = iter.into_iter().map(Into::into).collect();
        self.fields.extend(str_fields);
        self
    }

    pub fn add_where_in(&mut self, field: String, values: Vec<String>) -> &mut Self {
        self.filters.push(Filter {
            key: field,
            symbol: String::new(),
            value : format!("= ({})", values.join(",")),
        });

        self
    }

    pub fn add_where<L: Into<String>, R: Into<String>>(
        &mut self,
        field: L,
        equality: Equality,
        clause: R,
    ) -> &mut Self {
        self.filters.push(Filter {
            key: field.into(),
            symbol: equality.to_string(),
            value: clause.into(),
        });
        self
    }

    pub fn limit(&mut self, limit: usize) -> &mut Self {
        self.limit = limit;
        self
    }

    pub fn search<S: Into<String>>(&mut self, search: S) -> &mut Self {
        self.search = search.into();
        self
    }

    pub fn sort_by<S: Into<String>>(&mut self, field: S, order: OrderBy) -> &mut Self {
        self.sort = (field.into(), order.to_string());
        self
    }

    pub(crate) fn build(&self, api_key: &str, url: &str) -> surf::Request<impl HttpClient> {
        let body = &self.build_body();

        let mut req =
            surf::Request::new(http::Method::GET, Url::from_str(url).unwrap()).body_bytes(body);

        req.headers().insert(HEADER_KEY_NAME, api_key);
        req.headers().insert("content-type", "application/text");

        req
    }

    pub(crate) fn build_body(&self) -> Vec<u8> {
        let fields = self
            .fields
            .iter()
            .enumerate()
            .fold(String::new(), |mut acc, (i, v)| {
                acc.push_str(v);
                let separator = match i {
                    _ if i == (self.fields.len() - 1) => ";",
                    _ => ",",
                };
                acc.push_str(separator);
                acc
            });

        let filter_clause =
            self.filters
                .iter()
                .enumerate()
                .fold(String::new(), |mut acc, (i, filter)| {
                    if i == 0 {
                        acc.push_str("where ")
                    }
                    if i != 0 {
                        acc.push_str(" & ")
                    };

                    acc.push_str(&format!(
                        "{} {} {}",
                        filter.key,
                        filter.symbol,
                        filter.value
                    ));

                    if i == (self.filters.len() - 1) {
                        acc.push_str(";");
                    }
                    acc
                });

        self.format_body_parts(fields, filter_clause)
            .as_bytes()
            .to_vec()
    }

    fn format_body_parts(&self, fields: String, filters: String) -> String {
        let mut order = String::new();

        let mut body = format!("fields {}", fields);

        if !str::is_empty(&self.search) {
            body = format!("{} search \"{}\";", body, self.search);
        }

        if self.filters.len() > 0 {
            body = format!("{} {}", body, filters);
        }

        if !str::is_empty(&self.sort.0) {
            order.push_str(&format!("sort {} {}", self.sort.0, self.sort.1));
            body = format!("{} {}", body, order);
        }

        body = format!("{} limit {};", body, self.limit);
        println!("{}", body);
        body
    }
}

#[test]
fn request_builder_with_all_fields() {
    let mut builder = RequestBuilder::new();

    builder.all_fields();

    let body = builder.build_body();

    assert_eq!("fields *;", String::from_utf8_lossy(&body).to_owned());
}

#[test]
fn request_builder_with_fields_and_where_clause_body_build() {
    let mut builder = RequestBuilder::new();

    builder
        .add_field("name")
        .add_field("involved_companies")
        .add_where("name", Equality::Equal, "Conan")
        .add_where("id", Equality::Lower, 39047.to_string());

    let body = builder.build_body();

    assert_eq!(
        "fields name,involved_companies; where id < 39047 & name = Conan;",
        String::from_utf8_lossy(&body).to_owned()
    );
}

#[test]
fn request_builder_with_fields__where_clause_and_sort_asc_body_build() {
    let mut builder = RequestBuilder::new();

    builder
        .add_field("name")
        .add_field("involved_companies")
        .add_where("name", Equality::Equal, "Conan")
        .add_where("id", Equality::Equal, 39047.to_string())
        .sort_by("name", OrderBy::Ascending);

    let body = builder.build_body();

    assert_eq!(
        "fields name,involved_companies; where id = 39047 & name = Conan; sort name asc",
        String::from_utf8_lossy(&body).to_owned()
    );
}

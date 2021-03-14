table! {
    boards (name) {
        name -> Text,
        wait_time -> Int8,
        full_images -> Bool,
        last_modified -> Int8,
        archive -> Bool,
    }
}

table! {
    images (md5) {
        md5 -> Text,
        thumbnail -> Bool,
        full_image -> Bool,
    }
}

table! {
    posts (board, no) {
        board -> Varchar,
        no -> Int8,
        resto -> Int8,
        sticky -> Int8,
        closed -> Int8,
        now -> Text,
        time -> Int8,
        name -> Text,
        trip -> Text,
        id -> Varchar,
        capcode -> Text,
        country -> Varchar,
        country_name -> Text,
        sub -> Text,
        com -> Text,
        tim -> Int8,
        filename -> Text,
        ext -> Text,
        fsize -> Int8,
        md5 -> Text,
        w -> Int8,
        h -> Int8,
        tn_w -> Int8,
        tn_h -> Int8,
        filedeleted -> Int8,
        spoiler -> Int8,
        custom_spoiler -> Int8,
        replies -> Int8,
        images -> Int8,
        bumplimit -> Int8,
        imagelimit -> Int8,
        tag -> Text,
        semantic_url -> Text,
        since4pass -> Int8,
        unique_ips -> Int8,
        m_img -> Int8,
        archived -> Int8,
        archived_on -> Int8,
    }
}

allow_tables_to_appear_in_same_query!(
    boards,
    images,
    posts,
);

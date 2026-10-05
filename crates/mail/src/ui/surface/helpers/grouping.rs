use crate::ui::{mail_local_now, mail_local_timestamp};

const MAIL_GROUP_TODAY: &str = "Today";
const MAIL_GROUP_YESTERDAY: &str = "Yesterday";
const MAIL_GROUP_THIS_WEEK: &str = "Earlier This Week";
const MAIL_GROUP_THIS_MONTH: &str = "Earlier This Month";
const MAIL_GROUP_OLDER: &str = "Older";

pub(crate) fn mail_section_group(received_at: &str) -> &'static str {
    let Some(timestamp) = mail_local_timestamp(received_at) else {
        return MAIL_GROUP_OLDER;
    };
    let now = mail_local_now();
    let day_delta = (now.date().to_julian_day() - timestamp.date().to_julian_day()).max(0);
    if day_delta == 0 {
        MAIL_GROUP_TODAY
    } else if day_delta == 1 {
        MAIL_GROUP_YESTERDAY
    } else if day_delta < 7 {
        MAIL_GROUP_THIS_WEEK
    } else if now.date().month() == timestamp.date().month()
        && now.date().year() == timestamp.date().year()
    {
        MAIL_GROUP_THIS_MONTH
    } else {
        MAIL_GROUP_OLDER
    }
}

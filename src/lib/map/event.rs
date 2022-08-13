use crate::lib::time::time::Time;


pub struct Event
{
    time_activation: Time, 
    next_event: Option<i32>,
    if_executed: Option<i32>,
    executed: bool,

}
impl Event
{
    fn empty() -> Self
    {
        Self
        {
            time_activation: Time { minutes: 0 },
            next_event: None,
            if_executed: None,
            executed: false,

        }
    }
}